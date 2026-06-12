//! Tool method implementations for BotticelliServer.
//!
//! All `#[tool]` methods are declared without feature gates so `#[tool_router]`
//! can generate a complete routing table. Feature-gated functionality is
//! handled inside method bodies with `#[cfg(...)]` blocks.

use super::BotticelliServer;
use crate::tools::narrative_validation_helpers::{
    auto_fix_common_issues, format_toml, format_validation_result,
};
use botticelli_narrative::{
    TomlNarrativeFile,
    validator::{ValidationConfig, validate_narrative_toml, validate_narrative_toml_with_config},
};
use elicitation::{DynamicToolRegistry, ElicitJson as _, ElicitServer, elicit_tools};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content};
use rmcp::service::{Peer, RoleServer};
use rmcp::{ErrorData as RmcpError, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

// ============================================================================
// Input Types
// ============================================================================

/// Input for the echo tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EchoInput {
    /// Message to echo back.
    pub message: String,
}

/// Input for validating a narrative TOML.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateNarrativeInput {
    /// TOML content to validate.
    pub content: Option<String>,
    /// Path to a TOML file to validate.
    pub file_path: Option<String>,
    /// Check that media and nested narrative files exist.
    #[serde(default)]
    pub validate_files: bool,
    /// Warn on unknown model names.
    #[serde(default = "bool_true")]
    pub validate_models: bool,
    /// Warn about unused resources.
    #[serde(default = "bool_true")]
    pub warn_unused: bool,
    /// Treat warnings as errors.
    #[serde(default)]
    pub strict: bool,
}

#[instrument]
fn bool_true() -> bool {
    true
}

/// Input for saving a narrative to a file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SaveNarrativeInput {
    /// Narrative TOML content to save.
    pub narrative_toml: String,
    /// Destination file path (must end in .toml).
    pub file_path: String,
    /// Allow overwriting an existing file.
    #[serde(default)]
    pub overwrite: bool,
}

/// Input for modifying an existing narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyNarrativeInput {
    /// Existing narrative TOML.
    pub narrative_toml: String,
    /// Natural language description of the change.
    pub modification: String,
    /// Optional path to save the modified narrative.
    pub save_to: Option<String>,
}

/// Input for generating text with an LLM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateInput {
    /// Prompt to send to the LLM.
    pub prompt: String,
    /// Model identifier (e.g. "gemini-2.0-flash-exp", "claude-sonnet-4-6").
    pub model: Option<String>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Sampling temperature (0.0–2.0).
    pub temperature: Option<f32>,
    /// Optional system prompt.
    pub system_prompt: Option<String>,
}

/// Input for executing a single narrative act.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteActInput {
    /// Prompt for the act.
    pub prompt: String,
    /// Model identifier.
    pub model: String,
    /// Maximum tokens.
    pub max_tokens: Option<u32>,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// Optional system prompt.
    pub system_prompt: Option<String>,
}

/// Input for executing a full narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteNarrativeInput {
    /// Narrative TOML content.
    pub narrative_toml: Option<String>,
    /// Path to a narrative TOML file.
    pub file_path: Option<String>,
    /// User prompt / initial context.
    pub prompt: String,
    /// Model override.
    pub model: Option<String>,
    /// Temperature override.
    pub temperature: Option<f32>,
    /// Maximum tokens per act.
    pub max_tokens: Option<u32>,
}

/// Input for scene creation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateSceneInput {
    /// Narrative ID.
    pub narrative_id: String,
    /// Scene name.
    pub scene_name: String,
    /// Optional scene description.
    pub description: Option<String>,
}

/// Input for listing scenes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListScenesInput {
    /// Narrative ID.
    pub narrative_id: String,
}

/// Input for updating a scene.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateSceneInput {
    /// Narrative ID.
    pub narrative_id: String,
    /// Scene ID to update.
    pub scene_id: String,
    /// New scene name.
    pub scene_name: Option<String>,
    /// New description.
    pub description: Option<String>,
}

/// Input for deleting a scene.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteSceneInput {
    /// Narrative ID.
    pub narrative_id: String,
    /// Scene ID to delete.
    pub scene_id: String,
}

/// Input for writing a pre-built narrative file to disk.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteNarrativeFileInput {
    /// Fully-specified narrative file structure (serialized to TOML by the server).
    pub narrative: TomlNarrativeFile,
    /// Destination file path (must end in .toml).
    pub file_path: String,
    /// Allow overwriting an existing file.
    #[serde(default)]
    pub overwrite: bool,
}

/// Input for exporting metrics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportMetricsInput {
    /// Output format: "prometheus" or "summary".
    pub format: Option<String>,
}

/// Shared input for per-backend LLM generation tools.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateLlmInput {
    /// Prompt to send.
    pub prompt: String,
    /// Model identifier (uses backend default if omitted).
    pub model: Option<String>,
    /// Maximum tokens.
    pub max_tokens: Option<u32>,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// Optional system prompt.
    pub system_prompt: Option<String>,
}

/// Input for Discord post message.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiscordPostMessageInput {
    /// Discord channel ID.
    pub channel_id: String,
    /// Message content (max 2000 chars).
    pub content: String,
}

/// Input for Discord get messages.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiscordGetMessagesInput {
    /// Discord channel ID.
    pub channel_id: String,
    /// Number of messages to fetch (1–100).
    pub limit: Option<u32>,
}

/// Input for Discord get guild info.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiscordGetGuildInfoInput {
    /// Discord guild ID.
    pub guild_id: String,
}

/// Input for Discord get channels.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiscordGetChannelsInput {
    /// Discord guild ID.
    pub guild_id: String,
}

// ============================================================================
// Helpers
// ============================================================================

#[instrument(skip(value))]
fn json_ok(value: Value) -> Result<CallToolResult, RmcpError> {
    let text = serde_json::to_string_pretty(&value)
        .map_err(|e| RmcpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

#[instrument(skip(e))]
fn mcp_err(e: impl std::fmt::Display) -> RmcpError {
    RmcpError::internal_error(e.to_string(), None)
}

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
#[instrument(skip(system_prompt))]
fn build_llm_json(
    prompt: &str,
    model: &str,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    system_prompt: Option<&str>,
) -> Value {
    json!({
        "prompt": prompt,
        "model": model,
        "max_tokens": max_tokens.unwrap_or(1024),
        "temperature": temperature.unwrap_or(1.0),
        "system_prompt": system_prompt,
    })
}

// ============================================================================
// Server impl — tool_router block
// ============================================================================

#[tool_router]
impl BotticelliServer {
    /// Creates a new Botticelli MCP server instance.
    #[instrument]
    pub fn new() -> Self {
        info!("Creating Botticelli MCP server");

        #[cfg(feature = "gemini")]
        let gemini = match botticelli_models::GeminiClient::new() {
            Ok(c) => {
                info!("Gemini client initialized");
                Some(Arc::new(c))
            }
            Err(e) => {
                warn!(error = %e, "Gemini unavailable");
                None
            }
        };

        #[cfg(feature = "anthropic")]
        let anthropic = match std::env::var("ANTHROPIC_API_KEY") {
            Ok(key) => {
                info!("Anthropic client initialized");
                Some(Arc::new(botticelli_models::AnthropicClient::new(
                    key,
                    "claude-sonnet-4-6".to_string(),
                )))
            }
            Err(_) => {
                warn!("Anthropic unavailable: ANTHROPIC_API_KEY not set");
                None
            }
        };

        #[cfg(feature = "ollama")]
        let ollama = match botticelli_models::OllamaClient::new("llama3.2") {
            Ok(c) => {
                info!("Ollama client initialized");
                Some(Arc::new(c))
            }
            Err(e) => {
                warn!(error = %e, "Ollama unavailable");
                None
            }
        };

        #[cfg(feature = "huggingface")]
        let huggingface = std::env::var("HUGGINGFACE_MODEL")
            .ok()
            .and_then(|m| botticelli_models::HuggingFaceDriver::new(m).ok())
            .map(|c| {
                info!("HuggingFace driver initialized");
                Arc::new(c)
            });

        #[cfg(feature = "groq")]
        let groq = std::env::var("GROQ_MODEL")
            .ok()
            .and_then(|m| botticelli_models::GroqDriver::new(m).ok())
            .map(|c| {
                info!("Groq driver initialized");
                Arc::new(c)
            });

        #[cfg(feature = "discord")]
        let discord_client = match std::env::var("DISCORD_TOKEN") {
            Ok(token) => {
                info!("Discord client initialized");
                Some(Arc::new(crate::tools::discord::DiscordClient::new(token)))
            }
            Err(_) => {
                warn!("Discord unavailable: DISCORD_TOKEN not set");
                None
            }
        };

        Self {
            tool_router: Self::tool_router(),
            dynamic: DynamicToolRegistry::new(),
            metrics: Arc::new(crate::PrometheusMetrics::new()),
            #[cfg(feature = "gemini")]
            gemini,
            #[cfg(feature = "anthropic")]
            anthropic,
            #[cfg(feature = "ollama")]
            ollama,
            #[cfg(feature = "huggingface")]
            huggingface,
            #[cfg(feature = "groq")]
            groq,
            #[cfg(feature = "discord")]
            discord_client,
        }
    }

    // ========================================================================
    // Core tools
    // ========================================================================

    /// Echo a message back (for testing the MCP connection).
    #[instrument(skip(self))]
    #[tool(description = "Echoes back the input message. Useful for testing the MCP connection.")]
    pub async fn echo(
        &self,
        Parameters(req): Parameters<EchoInput>,
    ) -> Result<CallToolResult, RmcpError> {
        debug!(message = %req.message, "Echo called");
        json_ok(json!({
            "echo": req.message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// Returns information about this server.
    #[instrument(skip(self))]
    #[tool(
        description = "Returns information about the Botticelli MCP server, including version and capabilities."
    )]
    pub async fn get_server_info(&self) -> Result<CallToolResult, RmcpError> {
        debug!("Server info called");
        json_ok(json!({
            "name": "Botticelli MCP Server",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Model Context Protocol server for the Botticelli LLM orchestration platform",
            "capabilities": { "tools": true, "resources": false, "prompts": false },
        }))
    }

    // ========================================================================
    // Narrative creation
    // ========================================================================

    /// Create a narrative by presenting the full schema and asking for a JSON blob.
    #[instrument(skip(self, peer))]
    #[tool(
        description = "Create a new narrative TOML. Presents the full TomlNarrativeFile \
        JSON schema, asks the agent to supply a matching JSON object, then returns the \
        result serialized as TOML."
    )]
    pub async fn create_narrative(
        &self,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, RmcpError> {
        info!("Starting narrative elicitation (ElicitJson)");
        let server = ElicitServer::new(peer);
        let narrative = TomlNarrativeFile::elicit_json(&server)
            .await
            .map_err(mcp_err)?;
        let toml = toml::to_string_pretty(&narrative).map_err(mcp_err)?;
        info!(bytes = toml.len(), "Narrative elicitation complete");
        Ok(CallToolResult::success(vec![Content::text(toml)]))
    }

    // Elicitation builder tools — expose TomlNarrativeFile schema-driven construction
    // directly through the MCP protocol. The calling agent fills in each field via
    // the elicitation/create protocol and receives the completed struct as JSON.
    elicit_tools! { TomlNarrativeFile }

    // ========================================================================
    // Narrative manipulation
    // ========================================================================

    /// Validate a narrative TOML string or file.
    #[instrument(skip(self))]
    #[tool(
        description = "Validate a narrative TOML file or string. Checks syntax, structure, \
        references, model names, and circular dependencies."
    )]
    pub async fn validate_narrative(
        &self,
        Parameters(req): Parameters<ValidateNarrativeInput>,
    ) -> Result<CallToolResult, RmcpError> {
        debug!("Validating narrative");

        let toml_content = match (req.content.as_deref(), req.file_path.as_deref()) {
            (Some(c), _) => c.to_string(),
            (None, Some(path)) => std::fs::read_to_string(path).map_err(|e| {
                RmcpError::invalid_params(format!("Failed to read '{}': {}", path, e), None)
            })?,
            (None, None) => {
                return Err(RmcpError::invalid_params(
                    "Either 'content' or 'file_path' must be provided",
                    None,
                ));
            }
        };

        let config = ValidationConfig {
            validate_nested_narratives: req.validate_files,
            validate_media_files: req.validate_files,
            warn_unknown_models: req.validate_models,
            warn_unused_resources: req.warn_unused,
            base_dir: req
                .file_path
                .as_deref()
                .and_then(|p| PathBuf::from(p).parent().map(|d| d.to_path_buf())),
        };

        let result = validate_narrative_toml_with_config(&toml_content, &config);

        let errors: Vec<Value> = result
            .errors
            .iter()
            .map(|e| {
                json!({
                    "kind": format!("{:?}", e.kind),
                    "message": e.message,
                    "suggestion": e.suggestion,
                    "location": e.location.as_ref().map(|loc| json!({
                        "line": loc.line, "column": loc.column, "section": loc.section,
                    })),
                })
            })
            .collect();

        let warnings: Vec<Value> = result
            .warnings
            .iter()
            .map(|w| {
                json!({
                    "kind": format!("{:?}", w.kind),
                    "message": w.message,
                    "location": w.location.as_ref().map(|loc| json!({
                        "line": loc.line, "column": loc.column, "section": loc.section,
                    })),
                })
            })
            .collect();

        let is_valid = result.is_valid();
        let has_warnings = !result.warnings.is_empty();
        debug!(valid = is_valid, "Validation done");

        json_ok(json!({
            "valid": is_valid && (!req.strict || !has_warnings),
            "errors": errors,
            "warnings": warnings,
            "summary": format!("{} error(s), {} warning(s)", errors.len(), warnings.len()),
        }))
    }

    /// Save a narrative TOML to a file.
    #[instrument(skip(self))]
    #[tool(
        description = "Save a narrative TOML to a file. Validates the path and provides overwrite protection."
    )]
    pub async fn save_narrative(
        &self,
        Parameters(req): Parameters<SaveNarrativeInput>,
    ) -> Result<CallToolResult, RmcpError> {
        debug!(path = %req.file_path, "Saving narrative");

        let path = Path::new(&req.file_path);
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            return Err(RmcpError::invalid_params(
                "File path must end with .toml",
                None,
            ));
        }
        if path.exists() && !req.overwrite {
            return Err(RmcpError::invalid_params(
                format!(
                    "'{}' already exists. Set overwrite=true to replace it.",
                    req.file_path
                ),
                None,
            ));
        }
        if let Some(parent) = path.parent()
            && !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| mcp_err(format!("Failed to create directories: {}", e)))?;
            }
        tokio::fs::write(path, &req.narrative_toml)
            .await
            .map_err(|e| mcp_err(format!("Failed to write file: {}", e)))?;

        let absolute_path = std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| req.file_path.clone());

        debug!(path = %absolute_path, "Narrative saved");
        json_ok(json!({
            "status": "saved",
            "file_path": absolute_path,
            "size_bytes": req.narrative_toml.len(),
        }))
    }

    /// Modify an existing narrative based on natural language instructions.
    #[instrument(skip(self))]
    #[tool(
        description = "Modify an existing narrative based on natural language instructions. \
        Can add/remove/modify acts, change models, add resources, or update metadata."
    )]
    pub async fn modify_narrative(
        &self,
        Parameters(req): Parameters<ModifyNarrativeInput>,
    ) -> Result<CallToolResult, RmcpError> {
        debug!("Modifying narrative");

        let (mut modified_toml, mut changes) = crate::tools::modify_narrative::apply_modification(
            &req.narrative_toml,
            &req.modification,
        )
        .map_err(mcp_err)?;

        let (fixed_toml, fixes) = auto_fix_common_issues(&modified_toml);
        if !fixes.is_empty() {
            modified_toml = fixed_toml;
            changes.extend(fixes.iter().map(|f| format!("Auto-fix: {}", f)));
        }
        modified_toml = format_toml(&modified_toml);

        let validation = validate_narrative_toml(&modified_toml);

        let mut saved_to = None;
        if let Some(path) = &req.save_to {
            tokio::fs::write(path, &modified_toml)
                .await
                .map_err(|e| mcp_err(format!("Failed to save: {}", e)))?;
            saved_to = Some(path.clone());
        }

        let validation_json = format_validation_result(&validation);
        debug!(
            valid = validation.is_valid(),
            changes = changes.len(),
            "Narrative modified"
        );

        json_ok(json!({
            "toml": modified_toml,
            "validation": validation_json,
            "changes": changes,
            "saved_to": saved_to,
        }))
    }

    // ========================================================================
    // LLM generation — routing tool (always present)
    // ========================================================================

    /// Generate text using the configured LLM backend.
    #[instrument(skip(self))]
    #[tool(
        description = "Generate text using an LLM. Model prefix determines the backend: \
        'gemini*' → Gemini, 'claude*' → Anthropic, 'llama*'/'mistral*' → Ollama."
    )]
    pub async fn generate(
        &self,
        Parameters(req): Parameters<GenerateInput>,
    ) -> Result<CallToolResult, RmcpError> {
        debug!(model = ?req.model, "Generate called");
        let model = req.model.as_deref().unwrap_or("gemini-2.0-flash-exp");

        #[cfg(feature = "gemini")]
        if model.starts_with("gemini") || model.starts_with("models/gemini") {
            if let Some(ref client) = self.gemini {
                let input = build_llm_json(
                    &req.prompt,
                    model,
                    req.max_tokens,
                    req.temperature,
                    req.system_prompt.as_deref(),
                );
                let result =
                    crate::tools::generate_llm::execute_generation(client.as_ref(), input, model)
                        .await
                        .map_err(mcp_err)?;
                return json_ok(result);
            }
            return Err(RmcpError::internal_error(
                "Gemini not configured (check GEMINI_API_KEY)",
                None,
            ));
        }

        #[cfg(feature = "anthropic")]
        if model.starts_with("claude") {
            if let Some(ref client) = self.anthropic {
                let input = build_llm_json(
                    &req.prompt,
                    model,
                    req.max_tokens,
                    req.temperature,
                    req.system_prompt.as_deref(),
                );
                let result =
                    crate::tools::generate_llm::execute_generation(client.as_ref(), input, model)
                        .await
                        .map_err(mcp_err)?;
                return json_ok(result);
            }
            return Err(RmcpError::internal_error(
                "Anthropic not configured (check ANTHROPIC_API_KEY)",
                None,
            ));
        }

        #[cfg(feature = "ollama")]
        if model.starts_with("llama")
            || model.starts_with("mistral")
            || model.starts_with("codellama")
        {
            if let Some(ref client) = self.ollama {
                let input = build_llm_json(
                    &req.prompt,
                    model,
                    req.max_tokens,
                    req.temperature,
                    req.system_prompt.as_deref(),
                );
                let result =
                    crate::tools::generate_llm::execute_generation(client.as_ref(), input, model)
                        .await
                        .map_err(mcp_err)?;
                return json_ok(result);
            }
            return Err(RmcpError::internal_error("Ollama not configured", None));
        }

        Err(RmcpError::invalid_params(
            format!("Unknown or unsupported model prefix: '{}'", model),
            None,
        ))
    }

    /// Execute a single narrative act with a specified LLM.
    #[instrument(skip(self))]
    #[tool(description = "Execute a single narrative act with the specified prompt and model.")]
    pub async fn execute_act(
        &self,
        Parameters(req): Parameters<ExecuteActInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        ))]
        {
            debug!(model = %req.model, "Executing act");
            let driver = self.select_driver(&req.model)?;
            let input = build_llm_json(
                &req.prompt,
                &req.model,
                req.max_tokens,
                req.temperature,
                req.system_prompt.as_deref(),
            );
            let result =
                crate::tools::generate_llm::execute_generation(&*driver, input, &req.model)
                    .await
                    .map_err(mcp_err)?;
            return json_ok(result);
        }
        #[cfg(not(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        )))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "No LLM backend features compiled in",
                None,
            ))
        }
    }

    /// Execute a full narrative with a specified LLM backend.
    #[instrument(skip(self))]
    #[tool(
        description = "Execute a narrative TOML with an LLM backend. Runs each act in sequence."
    )]
    pub async fn execute_narrative(
        &self,
        Parameters(req): Parameters<ExecuteNarrativeInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        ))]
        {
            use botticelli_narrative::NarrativeExecutor;

            debug!("Executing narrative");

            let toml_content = match (req.narrative_toml.as_deref(), req.file_path.as_deref()) {
                (Some(c), _) => c.to_string(),
                (None, Some(path)) => std::fs::read_to_string(path).map_err(|e| {
                    RmcpError::invalid_params(format!("Failed to read '{}': {}", path, e), None)
                })?,
                (None, None) => {
                    return Err(RmcpError::invalid_params(
                        "Either 'narrative_toml' or 'file_path' must be provided",
                        None,
                    ));
                }
            };

            let narrative = botticelli_narrative::Narrative::from_toml_str(&toml_content, None)
                .map_err(|e| {
                    RmcpError::invalid_params(format!("Invalid narrative TOML: {}", e), None)
                })?;

            let model = req
                .model
                .as_deref()
                .or_else(|| narrative.metadata().model().as_deref())
                .unwrap_or("gemini-2.0-flash-exp");

            let driver = self.select_driver(model)?;
            let executor = NarrativeExecutor::new(driver);
            let results = executor.execute(&narrative).await.map_err(mcp_err)?;

            let act_count = results.act_executions().len();
            debug!(acts = act_count, "Narrative execution complete");
            return json_ok(json!({
                "status": "completed",
                "acts_executed": act_count,
                "results": results,
            }));
        }
        #[cfg(not(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        )))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "No LLM backend features compiled in",
                None,
            ))
        }
    }

    // ========================================================================
    // Per-backend LLM tools — always present but error without feature
    // ========================================================================

    /// Generate text using Google Gemini.
    #[instrument(skip(self))]
    #[tool(description = "Generate text using Google Gemini. Requires GEMINI_API_KEY.")]
    pub async fn generate_gemini(
        &self,
        Parameters(req): Parameters<GenerateLlmInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(feature = "gemini")]
        {
            let client = self.gemini.as_ref().ok_or_else(|| {
                RmcpError::internal_error("Gemini not configured (check GEMINI_API_KEY)", None)
            })?;
            let model = req.model.as_deref().unwrap_or("gemini-2.0-flash-exp");
            let input = build_llm_json(
                &req.prompt,
                model,
                req.max_tokens,
                req.temperature,
                req.system_prompt.as_deref(),
            );
            return crate::tools::generate_llm::execute_generation(client.as_ref(), input, model)
                .await
                .map_err(mcp_err)
                .and_then(json_ok);
        }
        #[cfg(not(feature = "gemini"))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "gemini feature not compiled in",
                None,
            ))
        }
    }

    /// Generate text using Anthropic Claude.
    #[instrument(skip(self))]
    #[tool(description = "Generate text using Anthropic Claude. Requires ANTHROPIC_API_KEY.")]
    pub async fn generate_anthropic(
        &self,
        Parameters(req): Parameters<GenerateLlmInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(feature = "anthropic")]
        {
            let client = self.anthropic.as_ref().ok_or_else(|| {
                RmcpError::internal_error(
                    "Anthropic not configured (check ANTHROPIC_API_KEY)",
                    None,
                )
            })?;
            let model = req.model.as_deref().unwrap_or("claude-sonnet-4-6");
            let input = build_llm_json(
                &req.prompt,
                model,
                req.max_tokens,
                req.temperature,
                req.system_prompt.as_deref(),
            );
            return crate::tools::generate_llm::execute_generation(client.as_ref(), input, model)
                .await
                .map_err(mcp_err)
                .and_then(json_ok);
        }
        #[cfg(not(feature = "anthropic"))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "anthropic feature not compiled in",
                None,
            ))
        }
    }

    /// Generate text using Ollama (local models).
    #[instrument(skip(self))]
    #[tool(description = "Generate text using Ollama local models.")]
    pub async fn generate_ollama(
        &self,
        Parameters(req): Parameters<GenerateLlmInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(feature = "ollama")]
        {
            let client = self
                .ollama
                .as_ref()
                .ok_or_else(|| RmcpError::internal_error("Ollama not configured", None))?;
            let model = req.model.as_deref().unwrap_or("llama3.2");
            let input = build_llm_json(
                &req.prompt,
                model,
                req.max_tokens,
                req.temperature,
                req.system_prompt.as_deref(),
            );
            return crate::tools::generate_llm::execute_generation(client.as_ref(), input, model)
                .await
                .map_err(mcp_err)
                .and_then(json_ok);
        }
        #[cfg(not(feature = "ollama"))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "ollama feature not compiled in",
                None,
            ))
        }
    }

    /// Generate text using HuggingFace models.
    #[instrument(skip(self))]
    #[tool(description = "Generate text using HuggingFace models. Requires HUGGINGFACE_MODEL.")]
    pub async fn generate_huggingface(
        &self,
        Parameters(req): Parameters<GenerateLlmInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(feature = "huggingface")]
        {
            let client = self.huggingface.as_ref().ok_or_else(|| {
                RmcpError::internal_error(
                    "HuggingFace not configured (check HUGGINGFACE_MODEL)",
                    None,
                )
            })?;
            let model = req.model.as_deref().unwrap_or("default");
            let input = build_llm_json(
                &req.prompt,
                model,
                req.max_tokens,
                req.temperature,
                req.system_prompt.as_deref(),
            );
            return crate::tools::generate_llm::execute_generation(client.as_ref(), input, model)
                .await
                .map_err(mcp_err)
                .and_then(json_ok);
        }
        #[cfg(not(feature = "huggingface"))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "huggingface feature not compiled in",
                None,
            ))
        }
    }

    /// Generate text using Groq.
    #[instrument(skip(self))]
    #[tool(description = "Generate text using Groq. Requires GROQ_MODEL.")]
    pub async fn generate_groq(
        &self,
        Parameters(req): Parameters<GenerateLlmInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(feature = "groq")]
        {
            let client = self
                .groq
                .as_ref()
                .ok_or_else(|| RmcpError::internal_error("Groq not configured", None))?;
            let model = req.model.as_deref().unwrap_or("default");
            let input = build_llm_json(
                &req.prompt,
                model,
                req.max_tokens,
                req.temperature,
                req.system_prompt.as_deref(),
            );
            return crate::tools::generate_llm::execute_generation(client.as_ref(), input, model)
                .await
                .map_err(mcp_err)
                .and_then(json_ok);
        }
        #[cfg(not(feature = "groq"))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "groq feature not compiled in",
                None,
            ))
        }
    }

    // ========================================================================
    // Discord tools — always present but error without feature
    // ========================================================================

    /// Post a message to a Discord channel.
    #[instrument(skip(self))]
    #[tool(
        description = "Post a message to a Discord channel. Requires discord feature and DISCORD_TOKEN."
    )]
    pub async fn discord_post_message(
        &self,
        Parameters(req): Parameters<DiscordPostMessageInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(feature = "discord")]
        {
            let client = self.discord_client.as_ref().ok_or_else(|| {
                RmcpError::internal_error("Discord not configured (check DISCORD_TOKEN)", None)
            })?;
            if req.content.len() > 2000 {
                return Err(RmcpError::invalid_params(
                    "Content exceeds 2000 character limit",
                    None,
                ));
            }
            let body = json!({ "content": req.content });
            let response = client
                .post(&format!("/channels/{}/messages", req.channel_id), body)
                .await
                .map_err(mcp_err)?;
            return json_ok(json!({
                "status": "success",
                "message_id": response.get("id"),
                "channel_id": req.channel_id,
                "timestamp": response.get("timestamp"),
            }));
        }
        #[cfg(not(feature = "discord"))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "discord feature not compiled in",
                None,
            ))
        }
    }

    /// Fetch message history from a Discord channel.
    #[instrument(skip(self))]
    #[tool(
        description = "Fetch message history from a Discord channel. Requires discord feature and DISCORD_TOKEN."
    )]
    pub async fn discord_get_messages(
        &self,
        Parameters(req): Parameters<DiscordGetMessagesInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(feature = "discord")]
        {
            let client = self.discord_client.as_ref().ok_or_else(|| {
                RmcpError::internal_error("Discord not configured (check DISCORD_TOKEN)", None)
            })?;
            let limit = req.limit.unwrap_or(50).clamp(1, 100);
            let response = client
                .get(&format!(
                    "/channels/{}/messages?limit={}",
                    req.channel_id, limit
                ))
                .await
                .map_err(mcp_err)?;
            return json_ok(json!({
                "status": "success",
                "channel_id": req.channel_id,
                "messages": response,
            }));
        }
        #[cfg(not(feature = "discord"))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "discord feature not compiled in",
                None,
            ))
        }
    }

    /// Get information about a Discord guild (server).
    #[instrument(skip(self))]
    #[tool(
        description = "Get information about a Discord guild. Requires discord feature and DISCORD_TOKEN."
    )]
    pub async fn discord_get_guild_info(
        &self,
        Parameters(req): Parameters<DiscordGetGuildInfoInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(feature = "discord")]
        {
            let client = self.discord_client.as_ref().ok_or_else(|| {
                RmcpError::internal_error("Discord not configured (check DISCORD_TOKEN)", None)
            })?;
            let response = client
                .get(&format!("/guilds/{}", req.guild_id))
                .await
                .map_err(mcp_err)?;
            return json_ok(json!({ "status": "success", "guild": response }));
        }
        #[cfg(not(feature = "discord"))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "discord feature not compiled in",
                None,
            ))
        }
    }

    /// Get channels in a Discord guild.
    #[instrument(skip(self))]
    #[tool(
        description = "Get channels in a Discord guild. Requires discord feature and DISCORD_TOKEN."
    )]
    pub async fn discord_get_channels(
        &self,
        Parameters(req): Parameters<DiscordGetChannelsInput>,
    ) -> Result<CallToolResult, RmcpError> {
        #[cfg(feature = "discord")]
        {
            let client = self.discord_client.as_ref().ok_or_else(|| {
                RmcpError::internal_error("Discord not configured (check DISCORD_TOKEN)", None)
            })?;
            let response = client
                .get(&format!("/guilds/{}/channels", req.guild_id))
                .await
                .map_err(mcp_err)?;
            return json_ok(json!({
                "status": "success",
                "guild_id": req.guild_id,
                "channels": response,
            }));
        }
        #[cfg(not(feature = "discord"))]
        {
            let _ = req;
            Err(RmcpError::internal_error(
                "discord feature not compiled in",
                None,
            ))
        }
    }

    // ========================================================================
    // Scene tools
    // ========================================================================

    /// Create a new scene in a narrative.
    #[instrument(skip(self))]
    #[tool(description = "Create a new scene in a narrative.")]
    pub async fn create_scene(
        &self,
        Parameters(req): Parameters<CreateSceneInput>,
    ) -> Result<CallToolResult, RmcpError> {
        info!(narrative_id = %req.narrative_id, scene_name = %req.scene_name, "Creating scene");
        json_ok(json!({
            "success": true,
            "scene_id": format!("scene_{}", uuid::Uuid::new_v4()),
            "narrative_id": req.narrative_id,
            "name": req.scene_name,
            "description": req.description,
        }))
    }

    /// List all scenes in a narrative.
    #[instrument(skip(self))]
    #[tool(description = "List all scenes in a narrative.")]
    pub async fn list_scenes(
        &self,
        Parameters(req): Parameters<ListScenesInput>,
    ) -> Result<CallToolResult, RmcpError> {
        info!(narrative_id = %req.narrative_id, "Listing scenes");
        json_ok(json!({
            "success": true,
            "narrative_id": req.narrative_id,
            "scenes": [],
            "count": 0,
        }))
    }

    /// Update a scene in a narrative.
    #[instrument(skip(self))]
    #[tool(description = "Update a scene in a narrative.")]
    pub async fn update_scene(
        &self,
        Parameters(req): Parameters<UpdateSceneInput>,
    ) -> Result<CallToolResult, RmcpError> {
        info!(narrative_id = %req.narrative_id, scene_id = %req.scene_id, "Updating scene");
        json_ok(json!({
            "success": true,
            "narrative_id": req.narrative_id,
            "scene_id": req.scene_id,
            "updated": { "name": req.scene_name, "description": req.description },
        }))
    }

    /// Delete a scene from a narrative.
    #[instrument(skip(self))]
    #[tool(description = "Delete a scene from a narrative.")]
    pub async fn delete_scene(
        &self,
        Parameters(req): Parameters<DeleteSceneInput>,
    ) -> Result<CallToolResult, RmcpError> {
        info!(narrative_id = %req.narrative_id, scene_id = %req.scene_id, "Deleting scene");
        json_ok(json!({
            "success": true,
            "narrative_id": req.narrative_id,
            "scene_id": req.scene_id,
        }))
    }

    /// Serialize a TomlNarrativeFile to TOML and write it to disk.
    #[instrument(skip(self))]
    #[tool(
        description = "Write a fully-specified narrative structure to a TOML file. \
        The TOML is produced by the library's own serializer, guaranteeing structural validity."
    )]
    pub async fn write_narrative_file(
        &self,
        Parameters(req): Parameters<WriteNarrativeFileInput>,
    ) -> Result<CallToolResult, RmcpError> {
        debug!(path = %req.file_path, "Writing narrative file");

        let path = Path::new(&req.file_path);
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            return Err(RmcpError::invalid_params(
                "File path must end with .toml",
                None,
            ));
        }
        if path.exists() && !req.overwrite {
            return Err(RmcpError::invalid_params(
                format!(
                    "'{}' already exists. Set overwrite=true to replace it.",
                    req.file_path
                ),
                None,
            ));
        }
        if let Some(parent) = path.parent()
            && !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| mcp_err(format!("Failed to create directories: {}", e)))?;
            }

        let toml_content = toml::to_string_pretty(&req.narrative)
            .map_err(|e| mcp_err(format!("Failed to serialize narrative to TOML: {}", e)))?;

        tokio::fs::write(path, &toml_content)
            .await
            .map_err(|e| mcp_err(format!("Failed to write file: {}", e)))?;

        let absolute_path = std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| req.file_path.clone());

        info!(path = %absolute_path, bytes = toml_content.len(), "Narrative file written");
        json_ok(json!({
            "status": "written",
            "file_path": absolute_path,
            "size_bytes": toml_content.len(),
        }))
    }

    // ========================================================================
    // Metrics
    // ========================================================================

    /// Export execution metrics.
    #[instrument(skip(self))]
    #[tool(description = "Export execution metrics in Prometheus text format.")]
    pub async fn export_metrics(
        &self,
        Parameters(req): Parameters<ExportMetricsInput>,
    ) -> Result<CallToolResult, RmcpError> {
        let format = req.format.as_deref().unwrap_or("prometheus");
        debug!(format, "Exporting metrics");
        match format {
            "prometheus" => {
                let text = self.metrics.export_prometheus().map_err(mcp_err)?;
                json_ok(json!({ "format": "prometheus", "metrics": text }))
            }
            "summary" => {
                let s = self.metrics.summary().map_err(mcp_err)?;
                json_ok(json!({
                    "format": "summary",
                    "total_executions": s.total_executions,
                    "total_tokens": s.total_tokens,
                    "total_cost_usd": s.total_cost_usd,
                    "avg_duration_ms": s.avg_duration_ms,
                }))
            }
            _ => Err(RmcpError::invalid_params(
                format!(
                    "Unknown format '{}'. Use 'prometheus' or 'summary'.",
                    format
                ),
                None,
            )),
        }
    }
}

// ============================================================================
// LLM driver selection (only when at least one backend feature is enabled)
// ============================================================================

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
impl BotticelliServer {
    /// Select the appropriate LLM driver based on model prefix.
    #[instrument(skip(self))]
    pub(crate) fn select_driver(
        &self,
        model: &str,
    ) -> Result<Arc<dyn botticelli_interface::BotticelliDriver>, RmcpError> {
        #[cfg(feature = "gemini")]
        if model.starts_with("gemini") || model.starts_with("models/gemini") {
            return self
                .gemini
                .clone()
                .map(|d| d as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| RmcpError::internal_error("Gemini not configured", None));
        }

        #[cfg(feature = "anthropic")]
        if model.starts_with("claude") {
            return self
                .anthropic
                .clone()
                .map(|d| d as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| RmcpError::internal_error("Anthropic not configured", None));
        }

        #[cfg(feature = "ollama")]
        if model.starts_with("llama")
            || model.starts_with("mistral")
            || model.starts_with("codellama")
        {
            return self
                .ollama
                .clone()
                .map(|d| d as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| RmcpError::internal_error("Ollama not configured", None));
        }

        #[cfg(feature = "huggingface")]
        if model.contains('/') {
            return self
                .huggingface
                .clone()
                .map(|d| d as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| RmcpError::internal_error("HuggingFace not configured", None));
        }

        #[cfg(feature = "groq")]
        if model.contains("groq") || model.contains("llama3") {
            return self
                .groq
                .clone()
                .map(|d| d as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| RmcpError::internal_error("Groq not configured", None));
        }

        Err(RmcpError::invalid_params(
            format!("Unknown model prefix: '{}'", model),
            None,
        ))
    }
}
