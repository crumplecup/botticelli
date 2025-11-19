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
//! let request = GenerateRequest {
//!     messages: vec![Message {
//!         role: Role::User,
//!         content: vec![Input::Text("Hello".to_string())],
//!     }],
//!     ..Default::default()
//! };
//! let response = client.generate(&request).await?;
//! # Ok(())
//! # }
//! # }
//! ```

#[cfg(feature = "gemini")]
mod gemini;

#[cfg(feature = "gemini")]
pub use gemini::{
    ClientContent, ClientContentMessage, FunctionCall, FunctionResponse, GeminiClient,
    GeminiLiveClient, GenerationConfig, GoAway, InlineData, InlineDataPart, LiveRateLimiter,
    LiveSession, LiveToolCall, LiveToolCallCancellation, MediaChunk, ModelTurn, Part,
    RealtimeInput, RealtimeInputMessage, ServerContent, ServerMessage, SetupComplete, SetupConfig,
    SetupMessage, SystemInstruction, TextPart, TieredGemini, Tool, ToolResponse,
    ToolResponseMessage, Turn, UsageMetadata,
};
