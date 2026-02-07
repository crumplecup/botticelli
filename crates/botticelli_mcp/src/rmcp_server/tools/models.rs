//! Tool wrappers for model provider trait implementations.
//!
//! This module provides MCP tool wrappers for trait methods in botticelli_models
//! that cannot have `#[tool]` directly (async methods with references, trait implementations).
//!
//! TODO: Add #[tool_router] to register these tools with the MCP server.
//! Currently these functions are marked as tools but not registered, hence the
//! "never used" warnings. This will be fixed when we add the tool_router macro.

use anyhow::Result;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_models::{AnthropicClient, GeminiClient, GroqDriver, HuggingFaceDriver, OllamaClient};
use rmcp::tool;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use tracing::instrument;

// ============================================================================
// BotticelliDriver::generate() wrappers
// ============================================================================

/// Parameters for Gemini generate call.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeminiGenerateParams {
    /// Generate request (as JSON)
    pub request: serde_json::Value,
}

/// Wrapper for GeminiClient::generate() trait method.
#[tool(description = "Generate text using Gemini model")]
#[instrument(skip(params))]
pub async fn gemini_generate(
    params: GeminiGenerateParams,
) -> Result<GenerateResponse> {
    use botticelli_interface::BotticelliDriver;
    
    let request: GenerateRequest = serde_json::from_value(params.request)?;
    
    let client = GeminiClient::new()?;
    Ok(client.generate(&request).await?)
}

/// Parameters for Anthropic generate call.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnthropicGenerateParams {
    /// API key for authentication
    pub api_key: String,
    /// Model name
    pub model: String,
    /// Generate request (as JSON)
    pub request: serde_json::Value,
}

/// Wrapper for AnthropicClient::generate() trait method.
#[tool(description = "Generate text using Anthropic Claude model")]
#[instrument(skip(params))]
pub async fn anthropic_generate(
    params: AnthropicGenerateParams,
) -> Result<GenerateResponse> {
    use botticelli_interface::BotticelliDriver;
    
    let request: GenerateRequest = serde_json::from_value(params.request)?;
    
    let client = AnthropicClient::new(params.api_key, params.model);
    Ok(client.generate(&request).await?)
}

/// Parameters for Groq generate call.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GroqGenerateParams {
    /// Model name
    pub model: String,
    /// Generate request (as JSON)
    pub request: serde_json::Value,
}

/// Wrapper for GroqDriver::generate() trait method.
#[tool(description = "Generate text using Groq LPU model")]
#[instrument(skip(params))]
pub async fn groq_generate(
    params: GroqGenerateParams,
) -> Result<GenerateResponse> {
    use botticelli_interface::BotticelliDriver;
    
    let request: GenerateRequest = serde_json::from_value(params.request)?;
    
    let client = GroqDriver::new(params.model)?;
    Ok(client.generate(&request).await?)
}

/// Parameters for HuggingFace generate call.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HuggingFaceGenerateParams {
    /// Model name
    pub model: String,
    /// Generate request (as JSON)
    pub request: serde_json::Value,
}

/// Wrapper for HuggingFaceDriver::generate() trait method.
#[tool(description = "Generate text using HuggingFace Inference API")]
#[instrument(skip(params))]
pub async fn huggingface_generate(
    params: HuggingFaceGenerateParams,
) -> Result<GenerateResponse> {
    use botticelli_interface::BotticelliDriver;
    
    let request: GenerateRequest = serde_json::from_value(params.request)?;
    
    let client = HuggingFaceDriver::new(params.model)?;
    Ok(client.generate(&request).await?)
}

/// Parameters for Ollama generate call.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OllamaGenerateParams {
    /// Model name
    pub model: String,
    /// Generate request (as JSON)
    pub request: serde_json::Value,
}

/// Wrapper for OllamaClient::generate() trait method.
#[tool(description = "Generate text using Ollama local model")]
#[instrument(skip(params))]
pub async fn ollama_generate(
    params: OllamaGenerateParams,
) -> Result<GenerateResponse> {
    use botticelli_interface::BotticelliDriver;
    
    let request: GenerateRequest = serde_json::from_value(params.request)?;
    
    let client = OllamaClient::new(params.model)?;
    Ok(client.generate(&request).await?)
}

// ============================================================================
// TokenCounting trait wrappers
// ============================================================================

/// Parameters for token counting.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CountTokensParams {
    /// Provider name (gemini, anthropic, groq, huggingface, ollama)
    pub provider: String,
    /// Model name
    pub model: String,
    /// Text to count tokens for
    pub text: String,
}

/// Count tokens for text using specified provider.
///
/// TODO: This function is not yet registered with a tool_router, so it generates
/// a "never used" warning. This will be fixed when we add #[tool_router] to this module.
#[allow(dead_code)]
#[tool(description = "Count tokens in text using provider's tokenizer")]
#[instrument(skip(params))]
pub fn count_tokens(params: CountTokensParams) -> Result<usize> {
    use botticelli_interface::TokenCounting;
    
    match params.provider.as_str() {
        "gemini" => {
            let client = GeminiClient::new()?;
            Ok(client.count_tokens(&params.text)?)
        }
        "anthropic" => {
            let client = AnthropicClient::new(String::new(), params.model);
            Ok(client.count_tokens(&params.text)?)
        }
        "groq" => {
            let client = GroqDriver::new(params.model)?;
            Ok(client.count_tokens(&params.text)?)
        }
        "huggingface" => {
            let client = HuggingFaceDriver::new(params.model)?;
            Ok(client.count_tokens(&params.text)?)
        }
        "ollama" => {
            let client = OllamaClient::new(params.model)?;
            Ok(client.count_tokens(&params.text)?)
        }
        _ => Err(anyhow::anyhow!("Unsupported provider: {}", params.provider)),
    }
}
