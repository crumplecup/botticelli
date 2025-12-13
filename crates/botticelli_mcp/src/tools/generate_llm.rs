//! Multi-backend LLM generation tools.
//!
//! This module provides tools for text generation using multiple LLM backends.
//! Each backend is feature-gated and implemented explicitly.

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use crate::tools::McpTool;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_error::{McpError, McpResult};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use async_trait::async_trait;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_core::{GenerateRequest, Input, Message, Role};

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
use serde_json::{json, Value};

/// Common generation logic shared across all backends.
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
async fn execute_generation<D: BotticelliDriver>(
    driver: &D,
    input: Value,
    default_model: &str,
) -> McpResult<Value> {
    let prompt = input
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_input("Missing 'prompt'".to_string()))?;

    let model = input
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or(default_model);

    let max_tokens = input
        .get("max_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(1024) as u32;

    let temperature = input
        .get("temperature")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0) as f32;

    let system_prompt = input.get("system_prompt").and_then(|v| v.as_str());

    // Build messages
    let mut messages = Vec::new();

    if let Some(sys_prompt) = system_prompt {
        messages.push(
            Message::builder()
                .role(Role::User)
                .content(vec![Input::Text(sys_prompt.to_string())])
                .build()
                .map_err(|e| {
                    McpError::execution_failed(format!("Failed to build system message: {}", e))
                })?,
        );
    }

    messages.push(
        Message::builder()
            .role(Role::User)
            .content(vec![Input::Text(prompt.to_string())])
            .build()
            .map_err(|e| {
                McpError::execution_failed(format!("Failed to build message: {}", e))
            })?,
    );

    // Build request
    let request = GenerateRequest::builder()
        .model(Some(model.to_string()))
        .messages(messages)
        .max_tokens(Some(max_tokens))
        .temperature(Some(temperature))
        .build()
        .map_err(|e| McpError::execution_failed(format!("Failed to build request: {}", e)))?;

    // Execute
    let response = driver
        .generate(&request)
        .await
        .map_err(|e| McpError::execution_failed(format!("Generation failed: {}", e)))?;

    // Extract text
    let text = response
        .outputs()
        .iter()
        .filter_map(|output| {
            if let botticelli_core::Output::Text(t) = output {
                Some(t.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(json!({
        "status": "success",
        "model": model,
        "text": text,
    }))
}

// ============================================================================
// Gemini Tool
// ============================================================================

#[cfg(feature = "gemini")]
use botticelli_models::GeminiClient;

/// Tool for generating text with Google Gemini.
#[cfg(feature = "gemini")]
pub struct GenerateGeminiTool {
    client: GeminiClient,
}

#[cfg(feature = "gemini")]
impl GenerateGeminiTool {
    /// Creates a new Gemini generation tool.
    pub fn new() -> Result<Self, String> {
        let client = GeminiClient::new().map_err(|e| format!("Gemini client error: {}", e))?;
        Ok(Self { client })
    }
}

#[cfg(feature = "gemini")]
#[async_trait]
impl McpTool for GenerateGeminiTool {
    fn name(&self) -> &str {
        "generate_gemini"
    }

    fn description(&self) -> &str {
        "Generate text using Google Gemini models. Requires GEMINI_API_KEY environment variable."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to send to Gemini"
                },
                "model": {
                    "type": "string",
                    "description": "Gemini model to use",
                    "default": "gemini-2.0-flash-exp",
                    "enum": ["gemini-2.0-flash-exp", "gemini-1.5-pro", "gemini-1.5-flash"]
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens to generate",
                    "default": 1024
                },
                "temperature": {
                    "type": "number",
                    "description": "Sampling temperature (0.0-2.0)",
                    "default": 1.0
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        execute_generation(&self.client, input, "gemini-2.0-flash-exp").await
    }
}

// ============================================================================
// Anthropic Tool
// ============================================================================

#[cfg(feature = "anthropic")]
use botticelli_models::AnthropicClient;

/// Tool for generating text with Anthropic Claude.
#[cfg(feature = "anthropic")]
pub struct GenerateAnthropicTool {
    client: AnthropicClient,
}

#[cfg(feature = "anthropic")]
impl GenerateAnthropicTool {
    /// Creates a new Anthropic generation tool.
    pub fn new() -> Result<Self, String> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY environment variable not set".to_string())?;
        let model = "claude-3-5-sonnet-20241022".to_string();
        let client = AnthropicClient::new(api_key, model);
        Ok(Self { client })
    }
}

#[cfg(feature = "anthropic")]
#[async_trait]
impl McpTool for GenerateAnthropicTool {
    fn name(&self) -> &str {
        "generate_anthropic"
    }

    fn description(&self) -> &str {
        "Generate text using Anthropic Claude models. Requires ANTHROPIC_API_KEY environment variable."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to send to Claude"
                },
                "model": {
                    "type": "string",
                    "description": "Claude model to use",
                    "default": "claude-3-5-sonnet-20241022",
                    "enum": [
                        "claude-3-5-sonnet-20241022",
                        "claude-3-5-haiku-20241022",
                        "claude-3-opus-20240229"
                    ]
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens to generate",
                    "default": 1024
                },
                "temperature": {
                    "type": "number",
                    "description": "Sampling temperature (0.0-2.0)",
                    "default": 1.0
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        execute_generation(&self.client, input, "claude-3-5-sonnet-20241022").await
    }
}

// ============================================================================
// Ollama Tool
// ============================================================================

#[cfg(feature = "ollama")]
use botticelli_models::OllamaClient;

/// Tool for generating text with local Ollama models.
#[cfg(feature = "ollama")]
pub struct GenerateOllamaTool {
    client: OllamaClient,
}

#[cfg(feature = "ollama")]
impl GenerateOllamaTool {
    /// Creates a new Ollama generation tool.
    pub fn new() -> Result<Self, String> {
        let client =
            OllamaClient::new("llama3.2").map_err(|e| format!("Ollama client error: {}", e))?;
        Ok(Self { client })
    }
}

#[cfg(feature = "ollama")]
#[async_trait]
impl McpTool for GenerateOllamaTool {
    fn name(&self) -> &str {
        "generate_ollama"
    }

    fn description(&self) -> &str {
        "Generate text using local Ollama models. Requires Ollama server running."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to send to Ollama"
                },
                "model": {
                    "type": "string",
                    "description": "Ollama model to use",
                    "default": "llama3.2",
                    "enum": ["llama3.2", "mistral", "codellama"]
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens to generate",
                    "default": 1024
                },
                "temperature": {
                    "type": "number",
                    "description": "Sampling temperature (0.0-2.0)",
                    "default": 1.0
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        execute_generation(&self.client, input, "llama3.2").await
    }
}

// ============================================================================
// HuggingFace Tool
// ============================================================================

#[cfg(feature = "huggingface")]
use botticelli_models::HuggingFaceDriver;

/// Tool for generating text with HuggingFace models.
#[cfg(feature = "huggingface")]
pub struct GenerateHuggingFaceTool {
    client: HuggingFaceDriver,
}

#[cfg(feature = "huggingface")]
impl GenerateHuggingFaceTool {
    /// Creates a new HuggingFace generation tool.
    pub fn new() -> Result<Self, String> {
        let client = HuggingFaceDriver::new("meta-llama/Meta-Llama-3-8B-Instruct".to_string())
            .map_err(|e| format!("HuggingFace client error: {}", e))?;
        Ok(Self { client })
    }
}

#[cfg(feature = "huggingface")]
#[async_trait]
impl McpTool for GenerateHuggingFaceTool {
    fn name(&self) -> &str {
        "generate_huggingface"
    }

    fn description(&self) -> &str {
        "Generate text using HuggingFace models. Requires HUGGINGFACE_API_KEY environment variable."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to send to HuggingFace"
                },
                "model": {
                    "type": "string",
                    "description": "HuggingFace model to use",
                    "default": "meta-llama/Meta-Llama-3-8B-Instruct",
                    "enum": ["meta-llama/Meta-Llama-3-8B-Instruct"]
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens to generate",
                    "default": 1024
                },
                "temperature": {
                    "type": "number",
                    "description": "Sampling temperature (0.0-2.0)",
                    "default": 1.0
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        execute_generation(&self.client, input, "meta-llama/Meta-Llama-3-8B-Instruct").await
    }
}

// ============================================================================
// Groq Tool
// ============================================================================

#[cfg(feature = "groq")]
use botticelli_models::GroqDriver;

/// Tool for generating text with Groq models.
#[cfg(feature = "groq")]
pub struct GenerateGroqTool {
    client: GroqDriver,
}

#[cfg(feature = "groq")]
impl GenerateGroqTool {
    /// Creates a new Groq generation tool.
    pub fn new() -> Result<Self, String> {
        let client = GroqDriver::new("llama-3.3-70b-versatile".to_string())
            .map_err(|e| format!("Groq client error: {}", e))?;
        Ok(Self { client })
    }
}

#[cfg(feature = "groq")]
#[async_trait]
impl McpTool for GenerateGroqTool {
    fn name(&self) -> &str {
        "generate_groq"
    }

    fn description(&self) -> &str {
        "Generate text using Groq models. Requires GROQ_API_KEY environment variable."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to send to Groq"
                },
                "model": {
                    "type": "string",
                    "description": "Groq model to use",
                    "default": "llama-3.3-70b-versatile",
                    "enum": ["llama-3.3-70b-versatile", "mixtral-8x7b-32768"]
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens to generate",
                    "default": 1024
                },
                "temperature": {
                    "type": "number",
                    "description": "Sampling temperature (0.0-2.0)",
                    "default": 1.0
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        execute_generation(&self.client, input, "llama-3.3-70b-versatile").await
    }
}
