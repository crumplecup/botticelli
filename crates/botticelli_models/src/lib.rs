//! LLM provider integrations for Botticelli.
//!
//! This crate provides client implementations for various LLM providers, each behind
//! its own feature flag for flexible dependency management.
//!
//! # Available Providers
//!
//! - **Gemini** (Google) - Enable with `gemini` feature
//! - **Anthropic** (Claude) - Enable with `anthropic` feature
//! - **HuggingFace** - Enable with `huggingface` feature
//! - **Groq** - Enable with `groq` feature
//! - **Perplexity** - Enable with `perplexity` feature
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! botticelli-models = { version = "0.2", features = ["gemini"] }
//! ```
//!
//! ```no_run
//! # #[cfg(feature = "gemini")]
//! # {
//! use botticelli_models::GeminiClient;
//! use botticelli_interface::BotticelliDriver;
//! use botticelli_core::{GenerateRequest, Message, Role, Input};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GeminiClient::new()?;
//! let message = Message::new(Role::User, vec![Input::Text("Hello".to_string())]);
//! let request = GenerateRequest::new(vec![message]);
//! let response = client.generate(&request).await?;
//! # Ok(())
//! # }
//! # }
//! ```

mod gemini_model;
mod groq_model;
mod metrics;
mod model_family;
mod model_selector;
mod openai;
mod rate_limit_detection;
mod token_counting;

#[cfg(feature = "gemini")]
mod gemini;

#[cfg(feature = "ollama")]
mod ollama;

#[cfg(feature = "anthropic")]
mod anthropic;

#[cfg(feature = "huggingface")]
mod huggingface;

#[cfg(feature = "groq")]
mod groq;

// Export types defined in this crate
pub use gemini_model::GeminiModel;
pub use groq_model::GroqModel;
pub use metrics::{LlmMetrics, classify_error};
pub use model_family::ModelFamily;
pub use model_selector::{ModelBounds, ModelId, ModelSelector, SelectionStrategy};
pub use openai::{ChatMessage, ChatRequest, ChatResponse, OpenAIibleClient};
pub use rate_limit_detection::{RateLimitDetector, RateLimitStatus};
pub use token_counting::{claude_tokenizer, count_tokens_tiktoken, gpt_tokenizer};

#[cfg(feature = "gemini")]
pub use gemini::{
    ClientContent, ClientContentMessage, FunctionCall, FunctionResponse, GeminiClient,
    GeminiLiveClient, GenerationConfig, GenerationConfigBuilder, GoAway, InlineData, InlineDataPart, LiveRateLimiter,
    LiveSession, LiveToolCall, LiveToolCallCancellation, MediaChunk, ModelCapabilities, ModelTurn, Part,
    RealtimeInput, RealtimeInputMessage, ServerContent, ServerMessage, SetupComplete, SetupConfig,
    SetupMessage, SystemInstruction, TextPart, TieredGemini, Tool, ToolResponse,
    ToolResponseMessage, Turn, UsageMetadata,
};

#[cfg(feature = "ollama")]
pub use ollama::OllamaClient;

#[cfg(feature = "anthropic")]
pub use anthropic::{
    AnthropicClient, AnthropicContent, AnthropicContentBlock, AnthropicImageSource,
    AnthropicMessage, AnthropicMessageBuilder, AnthropicRequest, AnthropicRequestBuilder,
    AnthropicResponse, AnthropicResponseBuilder, AnthropicTool, AnthropicUsage,
};

#[cfg(feature = "huggingface")]
pub use huggingface::HuggingFaceDriver;

#[cfg(feature = "groq")]
pub use groq::GroqDriver;

#[cfg(test)]
pub mod test_doubles;
