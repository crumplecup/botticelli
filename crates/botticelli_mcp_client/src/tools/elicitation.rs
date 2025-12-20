//! Elicitation tool implementations for conversational narrative creation.

use crate::tools::registry_ops::GenericRegistry;
use crate::{McpClientError, McpClientErrorKind, McpClientResult, ToolHandler};
use async_trait::async_trait;
use botticelli_error::{McpError, McpErrorKind, McpResult};
use botticelli_interface::RegistryOperations;
use pmcp::{Content, ToolInfo};
use serde_json::{Value, json};
use uuid::Uuid;

/// Elicitation session state wrapper implementing RegistryOperations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, derive_getters::Getters)]
pub struct ElicitationSession {
    id: Uuid,
    state: Value,
}

impl ElicitationSession {
    /// Create a new session with generated ID.
    pub fn new(state: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            state,
        }
    }
}

impl RegistryOperations for ElicitationSession {
    type Key = Uuid;

    fn registry_key(&self) -> Self::Key {
        self.id
    }

    fn from_json_args(args: Value) -> McpResult<Self> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let state = args.get("state").cloned().unwrap_or(json!({}));

        Ok(Self { id, state })
    }

    fn to_json(&self) -> McpResult<Value> {
        serde_json::to_value(self)
            .map_err(|e| McpError::new(McpErrorKind::ExecutionError(e.to_string())))
    }

    fn update_from_json(&mut self, args: Value) -> McpResult<()> {
        if let Some(state) = args.get("state") {
            self.state = state.clone();
        }
        Ok(())
    }
}

/// Registry for managing active narrative elicitation sessions.
pub type ElicitationRegistry = GenericRegistry<ElicitationSession>;

/// Tool for creating a new narrative elicitation session.
#[derive(Debug, Clone, derive_new::new)]
pub struct CreateElicitationSessionTool {
    registry: ElicitationRegistry,
}

#[async_trait]
impl ToolHandler for CreateElicitationSessionTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "create_elicitation_session",
            Some(
                "Initialize a new conversational narrative elicitation session. \
                 Analyzes user description and returns session ID for tracking."
                    .to_string(),
            ),
            json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "User's description of what they want to create"
                    }
                },
                "required": ["description"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpClientResult<Vec<Content>> {
        let description = input
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing 'description'".to_string(),
                ))
            })?;

        tracing::debug!(description, "Creating elicitation session");

        // Analyze description
        let acts = extract_acts_from_description(description);
        let suggested_name = suggest_name_from_description(description);

        let complexity = if acts.len() == 1 {
            "simple"
        } else if acts.len() <= 3 {
            "moderate"
        } else {
            "complex"
        };

        // Initialize session state
        let state = json!({
            "description": description,
            "suggested_name": suggested_name,
            "acts": acts,
            "metadata": {}
        });

        let session = ElicitationSession {
            id: Uuid::new_v4(),
            state,
        };

        let session_id = self.registry.upsert(session)?;

        let result = json!({
            "session_id": session_id.to_string(),
            "suggested_name": suggested_name,
            "analysis": {
                "detected_acts": acts.iter().map(|a| a["name"].as_str().unwrap_or("unknown")).collect::<Vec<_>>(),
                "complexity": complexity,
                "act_count": acts.len()
            }
        });

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        }])
    }
}

/// Tool for setting narrative metadata during elicitation.
#[derive(Debug, Clone, derive_new::new)]
pub struct ElicitMetadataTool {
    registry: ElicitationRegistry,
}

#[async_trait]
impl ToolHandler for ElicitMetadataTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "elicit_metadata",
            Some("Set or update narrative metadata (name, description, defaults)".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Elicitation session UUID"
                    },
                    "name": {
                        "type": "string",
                        "description": "Narrative name"
                    },
                    "description": {
                        "type": "string",
                        "description": "Narrative description"
                    },
                    "default_model": {
                        "type": "string",
                        "description": "Default model for acts"
                    },
                    "default_temperature": {
                        "type": "number",
                        "description": "Default temperature (0.0-1.0)"
                    }
                },
                "required": ["session_id"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpClientResult<Vec<Content>> {
        let session_id = input
            .get("session_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Invalid session_id".to_string(),
                ))
            })?;

        let session = self.registry.get(&session_id)?.ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Session not found".to_string(),
            ))
        })?;

        // Update metadata fields if provided
        let mut state_value = session.state().clone();
        let mut metadata = state_value["metadata"]
            .as_object()
            .cloned()
            .unwrap_or_default();

        if let Some(name) = input.get("name").and_then(|v| v.as_str()) {
            metadata.insert("name".to_string(), json!(name));
        }

        if let Some(desc) = input.get("description").and_then(|v| v.as_str()) {
            metadata.insert("description".to_string(), json!(desc));
        }

        if let Some(model) = input.get("default_model").and_then(|v| v.as_str()) {
            metadata.insert("default_model".to_string(), json!(model));
        }

        if let Some(temp) = input.get("default_temperature").and_then(|v| v.as_f64()) {
            metadata.insert("default_temperature".to_string(), json!(temp));
        }

        state_value["metadata"] = json!(metadata);
        let updated_session = ElicitationSession {
            id: session_id,
            state: state_value,
        };
        self.registry.update(&session_id, updated_session)?;

        let result = json!({
            "session_id": session_id.to_string(),
            "metadata": metadata,
            "status": "updated"
        });

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        }])
    }
}

/// Tool for adding or updating acts during elicitation.
#[derive(Debug, Clone, derive_new::new)]
pub struct ElicitActTool {
    registry: ElicitationRegistry,
}

#[async_trait]
impl ToolHandler for ElicitActTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "elicit_act",
            Some("Add or update an act in the narrative being elicited".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Elicitation session UUID"
                    },
                    "act_name": {
                        "type": "string",
                        "description": "Act identifier"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "Act prompt"
                    },
                    "model": {
                        "type": "string",
                        "description": "Model override for this act"
                    },
                    "temperature": {
                        "type": "number",
                        "description": "Temperature override for this act"
                    }
                },
                "required": ["session_id", "act_name", "prompt"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpClientResult<Vec<Content>> {
        let session_id = input
            .get("session_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Invalid session_id".to_string(),
                ))
            })?;

        let act_name = input
            .get("act_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing 'act_name'".to_string(),
                ))
            })?;

        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing 'prompt'".to_string(),
                ))
            })?;

        let session = self.registry.get(&session_id)?.ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Session not found".to_string(),
            ))
        })?;

        // Update or add act
        let mut state_value = session.state().clone();
        let mut acts = state_value["acts"].as_array().cloned().unwrap_or_default();

        let mut act = json!({
            "name": act_name,
            "prompt": prompt
        });

        if let Some(model) = input.get("model").and_then(|v| v.as_str()) {
            act["model"] = json!(model);
        }

        if let Some(temp) = input.get("temperature").and_then(|v| v.as_f64()) {
            act["temperature"] = json!(temp);
        }

        // Replace if exists, otherwise add
        if let Some(pos) = acts.iter().position(|a| a["name"] == act_name) {
            acts[pos] = act;
        } else {
            acts.push(act);
        }

        state_value["acts"] = json!(acts);
        let updated_session = ElicitationSession {
            id: session_id,
            state: state_value,
        };
        self.registry.update(&session_id, updated_session)?;

        let result = json!({
            "session_id": session_id.to_string(),
            "act_name": act_name,
            "status": "updated",
            "total_acts": acts.len()
        });

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        }])
    }
}
/// Tool for executing narratives in carousel mode with multiple iterations.
#[derive(Debug, Clone, derive_new::new)]
pub struct ExecuteCarouselTool {
    registry: ElicitationRegistry,
}

#[async_trait]
impl ToolHandler for ExecuteCarouselTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "execute_carousel",
            Some("Execute a narrative in carousel mode with multiple iterations and budget management".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "narrative_toml": {
                        "type": "string",
                        "description": "Complete narrative TOML with carousel configuration"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional elicitation session ID to track execution state"
                    }
                },
                "required": ["narrative_toml"]
            }),
        )
    }

    #[tracing::instrument(skip(self), fields(tool = "execute_carousel"))]
    async fn execute(&self, arguments: Value) -> McpClientResult<Vec<Content>> {
        let narrative_toml = arguments
            .get("narrative_toml")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing required 'narrative_toml' field".to_string(),
                ))
            })?;

        // Get or create session for tracking carousel state
        let session_id = arguments
            .get("session_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        // Store initial carousel execution state
        let execution_state = json!({
            "narrative_toml": narrative_toml,
            "status": "initialized",
            "iterations": [],
            "started_at": chrono::Utc::now().to_rfc3339()
        });

        let session = ElicitationSession {
            id: session_id,
            state: execution_state,
        };

        let stored_id = self.registry.upsert(session)?;

        tracing::info!(
            session_id = %stored_id,
            "Carousel execution session created"
        );

        // Return session info for tracking
        Ok(vec![Content::Text {
            text: json!({
                "status": "session_created",
                "session_id": stored_id.to_string(),
                "message": "Carousel execution session initialized. Narrative parsing will happen on execution.",
                "next_step": "Use session_id to track execution progress"
            })
            .to_string(),
        }])
    }
}

/// Tool for finalizing elicitation and generating TOML.
#[derive(Debug, Clone, derive_new::new)]
pub struct FinalizeElicitationTool {
    registry: ElicitationRegistry,
}

#[async_trait]
impl ToolHandler for FinalizeElicitationTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "finalize_elicitation",
            Some("Complete the elicitation session and generate narrative TOML file".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Elicitation session UUID"
                    }
                },
                "required": ["session_id"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpClientResult<Vec<Content>> {
        let session_id = input
            .get("session_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Invalid session_id".to_string(),
                ))
            })?;

        let session = self.registry.remove(&session_id)?.ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Session not found".to_string(),
            ))
        })?;

        // Generate TOML from state
        let toml = generate_toml_from_state(session.state())?;

        let result = json!({
            "session_id": session_id.to_string(),
            "toml": toml,
            "status": "finalized"
        });

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        }])
    }
}

// Helper functions

fn extract_acts_from_description(description: &str) -> Vec<Value> {
    // Simple heuristic: look for numbered sections or "then"/"next" patterns
    let description_lower = description.to_lowercase();

    if description_lower.contains("then") || description_lower.contains("next") {
        // Multi-act narrative
        vec![
            json!({"name": "setup", "prompt": "Set up the scenario"}),
            json!({"name": "development", "prompt": "Develop the main content"}),
            json!({"name": "conclusion", "prompt": "Conclude the narrative"}),
        ]
    } else {
        // Single act
        vec![json!({"name": "main", "prompt": description})]
    }
}

fn suggest_name_from_description(description: &str) -> String {
    // Extract first few words as suggestion
    let words: Vec<&str> = description.split_whitespace().take(3).collect();
    words
        .join("_")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn generate_toml_from_state(state: &Value) -> McpClientResult<String> {
    let metadata = &state["metadata"];
    let acts = state["acts"].as_array().ok_or_else(|| {
        McpClientError::new(McpClientErrorKind::InvalidToolCall(
            "Missing acts".to_string(),
        ))
    })?;

    let name = metadata["name"].as_str().ok_or_else(|| {
        McpClientError::new(McpClientErrorKind::InvalidToolCall(
            "Missing name".to_string(),
        ))
    })?;

    let description = metadata["description"]
        .as_str()
        .unwrap_or("Generated narrative");

    let mut toml = String::new();

    // [narrative] section
    toml.push_str("[narrative]\n");
    toml.push_str(&format!("name = \"{}\"\n", name));
    toml.push_str(&format!("description = \"{}\"\n", description));

    if let Some(model) = metadata["default_model"].as_str() {
        toml.push_str(&format!("model = \"{}\"\n", model));
    }

    if let Some(temp) = metadata["default_temperature"].as_f64() {
        toml.push_str(&format!("temperature = {}\n", temp));
    }

    toml.push('\n');

    // [toc] section
    toml.push_str("[toc]\n");
    toml.push_str("order = [");
    for (i, act) in acts.iter().enumerate() {
        if i > 0 {
            toml.push_str(", ");
        }
        let act_name = act["name"].as_str().unwrap_or("unknown");
        toml.push_str(&format!("\"{}\"", act_name));
    }
    toml.push_str("]\n\n");

    // [acts] section
    toml.push_str("[acts]\n");
    for act in acts {
        let act_name = act["name"].as_str().unwrap_or("unknown");
        let prompt = act["prompt"].as_str().unwrap_or("");
        toml.push_str(&format!(
            "{} = \"{}\"\n",
            act_name,
            escape_toml_string(prompt)
        ));
    }

    Ok(toml)
}

/// Tool for creating a carousel narrative.
#[derive(Debug, Clone, derive_new::new)]
pub struct CreateCarouselTool {
    registry: ElicitationRegistry,
}

#[async_trait]
impl ToolHandler for CreateCarouselTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "create_carousel",
            Some(
                "Create a carousel narrative for iterative content generation. \
                 Carousels execute multiple iterations with budget constraints."
                    .to_string(),
            ),
            json!({
                "type": "object",
                "properties": {
                    "iterations": {
                        "type": "number",
                        "description": "Maximum number of iterations to execute"
                    },
                    "estimated_tokens_per_iteration": {
                        "type": "number",
                        "description": "Estimated tokens per iteration for budget planning"
                    },
                    "continue_on_error": {
                        "type": "boolean",
                        "description": "Whether to continue executing on errors"
                    },
                    "narrative_template": {
                        "type": "string",
                        "description": "Narrative TOML template or path to use for each iteration"
                    }
                },
                "required": ["iterations", "narrative_template"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, arguments: Value) -> McpClientResult<Vec<Content>> {
        let iterations: u32 = arguments["iterations"]
            .as_u64()
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "iterations must be a number".to_string(),
                ))
            })?
            .try_into()
            .map_err(|_| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "iterations out of range".to_string(),
                ))
            })?;

        let estimated_tokens = arguments
            .get("estimated_tokens_per_iteration")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000);

        let continue_on_error = arguments
            .get("continue_on_error")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let narrative_template = arguments["narrative_template"]
            .as_str()
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "narrative_template must be a string".to_string(),
                ))
            })?
            .to_string();

        // Create carousel configuration
        let carousel_config = json!({
            "iterations": iterations,
            "estimated_tokens_per_iteration": estimated_tokens,
            "continue_on_error": continue_on_error,
            "narrative_template": narrative_template,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        let session = ElicitationSession::new(carousel_config);
        let session_id = self.registry.upsert(session)?;

        tracing::info!(
            session_id = %session_id,
            iterations = iterations,
            "Created carousel configuration"
        );

        Ok(vec![Content::Text {
            text: format!(
                "Created carousel with ID: {}\nIterations: {}\nEstimated tokens per iteration: {}\nContinue on error: {}\nTemplate: {}",
                session_id, iterations, estimated_tokens, continue_on_error, narrative_template
            ),
        }])
    }
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
