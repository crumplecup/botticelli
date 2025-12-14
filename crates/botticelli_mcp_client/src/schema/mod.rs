//! Tool schema conversion for different LLM providers.

mod anthropic;
mod gemini;
mod groq;
mod huggingface;
mod ollama;
mod openai;

pub use anthropic::AnthropicToolSchema;
pub use gemini::GeminiToolSchema;
pub use groq::GroqToolSchema;
pub use huggingface::HuggingFaceToolSchema;
pub use ollama::OllamaToolSchema;
pub use openai::OpenAIToolSchema;

use serde_json::Value;

/// Common tool schema abstraction.
#[derive(Debug, Clone)]
pub struct ToolSchema {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    pub input_schema: Value,
}

impl ToolSchema {
    /// Create a new tool schema.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Trait for converting MCP tool schemas to provider-specific formats.
pub trait ToolSchemaConverter {
    /// The provider-specific tool format.
    type Output;

    /// Convert an MCP tool schema to the provider format.
    fn convert(schema: &ToolSchema) -> Self::Output;
}
