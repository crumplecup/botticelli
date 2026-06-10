//! Simple text generation tool for MCP.

use crate::tools::McpTool;
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use serde_json::{Value, json};

/// Tool for simple text generation.
///
/// This is a placeholder that describes how generation would work.
/// Full implementation requires LLM driver integration.
pub struct GenerateTool;

#[async_trait]
impl McpTool for GenerateTool {
    fn name(&self) -> &str {
        "generate"
    }

    fn description(&self) -> &str {
        "Generate text using an LLM. Specify the prompt and optionally the model. \
         Returns the generated text response."
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
                    "description": "Model to use (e.g., 'gemini-2.0-flash-exp', 'gpt-4o', 'claude-3-5-sonnet-20241022')",
                    "default": "gemini-2.0-flash-exp"
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens to generate",
                    "default": 1024
                },
                "temperature": {
                    "type": "number",
                    "description": "Sampling temperature (0.0-2.0)",
                    "default": 1.0,
                    "minimum": 0.0,
                    "maximum": 2.0
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt to set context"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'prompt'".to_string()))?;

        let model = input
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("gemini-2.0-flash-exp");

        let max_tokens = input
            .get("max_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(1024) as u32;

        let temperature = input
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let system_prompt = input.get("system_prompt").and_then(|v| v.as_str());

        // Return configuration that would be used for generation
        // Full implementation requires LLM driver integration
        Ok(json!({
            "status": "configured",
            "config": {
                "prompt": prompt,
                "model": model,
                "max_tokens": max_tokens,
                "temperature": temperature,
                "system_prompt": system_prompt,
            },
            "note": "Full generation requires LLM driver integration. This tool currently validates and prepares the request."
        }))
    }
}
