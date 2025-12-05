//! Execute a single narrative act tool.

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
use tracing::{debug, error, instrument};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_core::{GenerateRequest, Input, MessageBuilder, Output, Role};

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
use std::sync::Arc;

/// Tool for executing a single narrative act with a specified LLM backend.
///
/// This provides a simpler interface than full narrative execution for
/// one-off LLM calls during development or testing.
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
pub struct ExecuteActTool {
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
pub struct ExecuteActTool;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
impl ExecuteActTool {
    /// Create a new execute act tool with available drivers.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "gemini")]
            gemini_driver: botticelli_models::GeminiClient::new().ok().map(Arc::new),
            #[cfg(feature = "anthropic")]
            anthropic_driver: std::env::var("ANTHROPIC_API_KEY").ok().map(|key| {
                Arc::new(botticelli_models::AnthropicClient::new(
                    key,
                    "claude-3-5-sonnet-20241022".to_string(),
                ))
            }),
            #[cfg(feature = "ollama")]
            ollama_driver: botticelli_models::OllamaClient::new("llama3.2")
                .ok()
                .map(Arc::new),
            #[cfg(feature = "huggingface")]
            huggingface_driver: std::env::var("HUGGINGFACE_MODEL")
                .ok()
                .and_then(|model| botticelli_models::HuggingFaceDriver::new(model).ok())
                .map(Arc::new),
            #[cfg(feature = "groq")]
            groq_driver: std::env::var("GROQ_MODEL")
                .ok()
                .and_then(|model| botticelli_models::GroqDriver::new(model).ok())
                .map(Arc::new),
        }
    }

    /// Select the appropriate driver based on model prefix.
    fn select_driver(&self, model: &str) -> Result<Arc<dyn BotticelliDriver>, McpError> {
        #[cfg(feature = "gemini")]
        if model.starts_with("gemini") || model.starts_with("models/gemini") {
            return self
                .gemini_driver
                .clone()
                .map(|driver| driver as Arc<dyn BotticelliDriver>)
                .ok_or_else(|| McpError::BackendUnavailable("Gemini".into()));
        }

        #[cfg(feature = "anthropic")]
        if model.starts_with("claude") {
            return self
                .anthropic_driver
                .clone()
                .map(|driver| driver as Arc<dyn BotticelliDriver>)
                .ok_or_else(|| McpError::BackendUnavailable("Anthropic".into()));
        }

        #[cfg(feature = "ollama")]
        if model.starts_with("llama")
            || model.starts_with("mistral")
            || model.starts_with("codellama")
        {
            return self
                .ollama_driver
                .clone()
                .map(|driver| driver as Arc<dyn BotticelliDriver>)
                .ok_or_else(|| McpError::BackendUnavailable("Ollama".into()));
        }

        #[cfg(feature = "huggingface")]
        if model.contains("huggingface") || model.contains("/") {
            return self
                .huggingface_driver
                .clone()
                .map(|driver| driver as Arc<dyn BotticelliDriver>)
                .ok_or_else(|| McpError::BackendUnavailable("HuggingFace".into()));
        }

        #[cfg(feature = "groq")]
        if model.contains("groq") {
            return self
                .groq_driver
                .clone()
                .map(|driver| driver as Arc<dyn BotticelliDriver>)
                .ok_or_else(|| McpError::BackendUnavailable("Groq".into()));
        }

        Err(McpError::UnsupportedModel(model.to_string()))
    }
}

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
impl Default for ExecuteActTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
)))]
impl Default for ExecuteActTool {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
)))]
impl ExecuteActTool {
    /// Create a stub tool (no backends available).
    pub fn new() -> Self {
        Self
    }
}

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
#[async_trait]
impl McpTool for ExecuteActTool {
    fn name(&self) -> &str {
        "execute_act"
    }

    fn description(&self) -> &str {
        "Execute a single narrative act with the specified prompt and model configuration"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to send to the LLM"
                },
                "model": {
                    "type": "string",
                    "description": "Model identifier (e.g., 'gemini-pro', 'claude-3-sonnet', 'llama2')"
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens to generate (optional, default: 1024)",
                    "default": 1024
                },
                "temperature": {
                    "type": "number",
                    "description": "Sampling temperature 0.0-1.0 (optional, default: 0.7)",
                    "default": 0.7
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt to set context"
                }
            },
            "required": ["prompt", "model"]
        })
    }

    #[instrument(skip(self, input), fields(tool = "execute_act"))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        debug!("Executing act");

        // Extract parameters
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'prompt' parameter".into()))?;

        let model = input
            .get("model")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'model' parameter".into()))?;

        let max_tokens = input
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(1024) as u32;

        let temperature = input
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);

        let system_prompt = input.get("system_prompt").and_then(|v| v.as_str());

        debug!(
            prompt_len = prompt.len(),
            model,
            max_tokens,
            temperature,
            has_system = system_prompt.is_some(),
            "Executing act"
        );

        // Select driver
        let driver = self.select_driver(model)?;

        // Build messages
        let mut messages = Vec::new();

        if let Some(sys) = system_prompt {
            messages.push(
                MessageBuilder::default()
                    .role(Role::System)
                    .content(vec![Input::Text(sys.to_string())])
                    .build()
                    .map_err(|e| {
                        error!(error = ?e, "Failed to build system message");
                        McpError::ExecutionError(format!("Failed to build system message: {}", e))
                    })?,
            );
        }

        messages.push(
            MessageBuilder::default()
                .role(Role::User)
                .content(vec![Input::Text(prompt.to_string())])
                .build()
                .map_err(|e| {
                    error!(error = ?e, "Failed to build user message");
                    McpError::ExecutionError(format!("Failed to build user message: {}", e))
                })?,
        );

        // Build request
        let request = GenerateRequest::builder()
            .model(Some(model.to_string()))
            .messages(messages)
            .max_tokens(Some(max_tokens))
            .temperature(Some(temperature as f32))
            .build()
            .map_err(|e| {
                error!(error = ?e, "Failed to build request");
                McpError::ExecutionError(format!("Failed to build request: {}", e))
            })?;

        // Execute
        match driver.generate(&request).await {
            Ok(response) => {
                // Extract text from outputs
                let text = response
                    .outputs()
                    .iter()
                    .filter_map(|output| {
                        if let Output::Text(t) = output {
                            Some(t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                debug!(
                    response_len = text.len(),
                    "Act executed successfully"
                );

                Ok(json!({
                    "success": true,
                    "response": text,
                    "model": model
                }))
            }
            Err(e) => {
                error!(error = ?e, "Failed to execute act");
                Err(McpError::ExecutionError(format!(
                    "LLM execution failed: {}",
                    e
                )))
            }
        }
    }
}

#[cfg(not(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
)))]
#[async_trait]
impl McpTool for ExecuteActTool {
    fn name(&self) -> &str {
        "execute_act"
    }

    fn description(&self) -> &str {
        "Execute a single narrative act (no LLM backends enabled)"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: Value) -> McpResult<Value> {
        Err(McpError::BackendUnavailable("No LLM backends enabled. Enable at least one feature: gemini, anthropic, ollama, huggingface, or groq".into()))
    }
}
