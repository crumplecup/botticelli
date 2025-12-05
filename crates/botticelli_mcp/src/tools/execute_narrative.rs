//! Narrative execution tool for MCP.

use crate::tools::McpTool;
use crate::{McpError, McpResult};
use async_trait::async_trait;
use serde_json::{json, Value};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_interface::BotticelliDriver;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_narrative::NarrativeExecutor;

/// Tool for executing narrative TOML files with a specified LLM backend.
///
/// Loads a narrative, selects the appropriate LLM driver, and executes
/// the narrative acts in sequence with the given prompt.
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use std::sync::Arc;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use tracing::{debug, error, instrument};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
/// MCP tool for executing narrative TOML files with multi-backend LLM support.
pub struct ExecuteNarrativeTool {
    #[cfg(feature = "gemini")]
    gemini_driver: Option<Arc<botticelli_models::GeminiClient>>,
    #[cfg(feature = "anthropic")]
    anthropic_driver: Option<Arc<botticelli_models::AnthropicClient>>,
    #[cfg(feature = "ollama")]
    ollama_driver: Option<Arc<botticelli_models::OllamaClient>>,
    #[cfg(feature = "huggingface")]
    huggingface_driver: Option<Arc<botticelli_models::HuggingFaceDriver>>,
    #[cfg(feature = "groq")]
    groq_driver: Option<Arc<botticelli_models::GroqDriver>>,
}

/// Stub tool when no LLM backends are enabled.
#[cfg(not(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
)))]
pub struct ExecuteNarrativeTool;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
impl ExecuteNarrativeTool {
    /// Create a new narrative execution tool with available drivers.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "gemini")]
            gemini_driver: botticelli_models::GeminiClient::new().ok().map(Arc::new),
            #[cfg(feature = "anthropic")]
            anthropic_driver: std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .map(|key| Arc::new(botticelli_models::AnthropicClient::new(key, "claude-3-5-sonnet-20241022".to_string()))),
            #[cfg(feature = "ollama")]
            ollama_driver: botticelli_models::OllamaClient::new("llama3.2").ok().map(Arc::new),
            #[cfg(feature = "huggingface")]
            huggingface_driver: botticelli_models::HuggingFaceDriver::new("meta-llama/Meta-Llama-3-8B-Instruct".to_string()).ok().map(Arc::new),
            #[cfg(feature = "groq")]
            groq_driver: botticelli_models::GroqDriver::new("llama-3.3-70b-versatile".to_string()).ok().map(Arc::new),
        }
    }

    #[instrument(skip(self, driver), fields(file_path, narrative_name))]
    async fn execute_with_driver<D: BotticelliDriver + Clone>(
        &self,
        driver: D,
        file_path: &str,
        narrative_name: &str,
    ) -> McpResult<Value> {
        debug!("Starting narrative execution");
        let executor = NarrativeExecutor::new(driver);
        
        let execution = executor
            .execute_narrative_by_name(file_path, narrative_name)
            .await
            .map_err(|e| {
                error!(error = ?e, "Narrative execution failed");
                McpError::ToolExecutionFailed(format!("Narrative execution failed: {}", e))
            })?;
        
        debug!(act_count = execution.act_executions.len(), "Narrative execution completed");

        let acts: Vec<Value> = execution
            .act_executions
            .iter()
            .map(|act| {
                json!({
                    "act_name": &act.act_name,
                    "model": &act.model,
                    "response": &act.response,
                })
            })
            .collect();

        Ok(json!({
            "status": "success",
            "narrative_name": &execution.narrative_name,
            "act_count": execution.act_executions.len(),
            "acts": acts,
            "final_response": execution.act_executions.last().map(|a| a.response.as_str()).unwrap_or(""),
        }))
    }
}

#[cfg(not(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
)))]
impl ExecuteNarrativeTool {
    /// Create a stub tool when no backends are available.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExecuteNarrativeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpTool for ExecuteNarrativeTool {
    fn name(&self) -> &str {
        "execute_narrative"
    }

    fn description(&self) -> &str {
        "Execute a narrative from a TOML file using a specified LLM backend. \
         Processes all acts in sequence and returns the execution results. \
         Requires at least one LLM backend to be available (gemini, anthropic, ollama, etc.)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the narrative TOML file"
                },
                "prompt": {
                    "type": "string",
                    "description": "The prompt/input to process through the narrative"
                },
                "backend": {
                    "type": "string",
                    "description": "LLM backend to use (gemini, anthropic, ollama, huggingface, groq)",
                    "enum": ["gemini", "anthropic", "ollama", "huggingface", "groq"],
                    "default": "gemini"
                },
                "variables": {
                    "type": "object",
                    "description": "Optional variables to pass to the narrative",
                    "additionalProperties": {"type": "string"}
                }
            },
            "required": ["file_path", "prompt"]
        })
    }

    #[cfg(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    ))]
    #[instrument(skip(self, input), fields(tool = "execute_narrative"))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        debug!("Executing narrative tool");
        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'file_path'".to_string()))?;

        let _prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'prompt'".to_string()))?;

        let backend = input
            .get("backend")
            .and_then(|v| v.as_str())
            .unwrap_or("gemini");
        
        debug!(file_path, backend, "Processing narrative execution request");

        // Determine narrative name from file path (use filename without extension)
        let narrative_name = std::path::Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| McpError::InvalidInput("Invalid file path".to_string()))?;

        // Select backend and execute
        match backend {
            #[cfg(feature = "gemini")]
            "gemini" => {
                if let Some(driver) = self.gemini_driver.clone() {
                    self.execute_with_driver(driver, file_path, narrative_name).await
                } else {
                    Err(McpError::ToolExecutionFailed("Gemini backend not available (check GEMINI_API_KEY)".to_string()))
                }
            }
            #[cfg(feature = "anthropic")]
            "anthropic" => {
                if let Some(driver) = self.anthropic_driver.clone() {
                    self.execute_with_driver(driver, file_path, narrative_name).await
                } else {
                    Err(McpError::ToolExecutionFailed("Anthropic backend not available (check ANTHROPIC_API_KEY)".to_string()))
                }
            }
            #[cfg(feature = "ollama")]
            "ollama" => {
                if let Some(driver) = self.ollama_driver.clone() {
                    self.execute_with_driver(driver, file_path, narrative_name).await
                } else {
                    Err(McpError::ToolExecutionFailed("Ollama backend not available (check Ollama server)".to_string()))
                }
            }
            #[cfg(feature = "huggingface")]
            "huggingface" => {
                if let Some(driver) = self.huggingface_driver.clone() {
                    self.execute_with_driver(driver, file_path, narrative_name).await
                } else {
                    Err(McpError::ToolExecutionFailed("HuggingFace backend not available (check HUGGINGFACE_API_KEY)".to_string()))
                }
            }
            #[cfg(feature = "groq")]
            "groq" => {
                if let Some(driver) = self.groq_driver.clone() {
                    self.execute_with_driver(driver, file_path, narrative_name).await
                } else {
                    Err(McpError::ToolExecutionFailed("Groq backend not available (check GROQ_API_KEY)".to_string()))
                }
            }
            _ => Err(McpError::InvalidInput(format!("Unknown or unavailable backend: {}", backend))),
        }
    }

    #[cfg(not(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    )))]
    async fn execute(&self, _input: Value) -> McpResult<Value> {
        Err(McpError::ToolExecutionFailed(
            "Narrative execution requires at least one LLM backend feature (gemini, anthropic, ollama, huggingface, or groq)".to_string()
        ))
    }
}
