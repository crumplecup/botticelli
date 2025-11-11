//! boticelli: unified interface for bot-driver interaction with multiple LLM APIs.

#![forbid(unsafe_code)]
//!
//! # Design Philosophy
//!
//! This library uses a trait-based architecture where capabilities are exposed through
//! separate traits. This allows:
//! - Models to implement only what they support
//! - Compile-time checking of capabilities
//! - Clear API boundaries for different features
//!
//! # Core Traits
//!
//! - [`BoticelliDriver`] - Core trait all backends must implement (basic generation)
//!
//! # Capability Traits
//!
//! Optional traits that backends may implement:
//! - [`Streaming`] - Streaming response support
//! - [`Embeddings`] - Text embedding generation
//! - [`Vision`] - Image input support (multimodal vision)
//! - [`Audio`] - Audio input/output support (speech, audio generation)
//! - [`Video`] - Video input/output support
//! - [`DocumentProcessing`] - Document understanding (PDF, DOCX, etc.)
//! - [`ToolUse`] - Function/tool calling
//! - [`JsonMode`] - Structured JSON output
//! - [`TokenCounting`] - Token counting utilities
//! - [`BatchGeneration`] - Batch request processing
//! - [`Metadata`] - Model metadata and limits
//! - [`Health`] - Health check support
//!
//! # Example
//!
//! ```rust,ignore
//! use boticelli::{BoticelliDriver, Streaming, GenerateRequest};
//!
//! async fn process<T>(client: &T, req: &GenerateRequest)
//! where
//!     T: BoticelliDriver + Streaming,
//! {
//!     // Can use both core and streaming capabilities
//!     let stream = client.generate_stream(req).await.unwrap();
//!     // ...
//! }
//! ```

mod core;
mod error;
mod interface;
mod models;

#[cfg(feature = "database")]
pub mod database;

pub mod narrative;
pub mod rate_limit;

// Re-export core types
pub use core::{
    GenerateRequest, GenerateResponse, Input, MediaSource, Message, Output, Role, ToolCall,
};

// Re-export error types
pub use error::{BoticelliError, BoticelliErrorKind, BoticelliResult};

// Re-export model-specific error types
#[cfg(feature = "gemini")]
pub use models::{GeminiError, GeminiErrorKind};

// Re-export core trait
pub use interface::BoticelliDriver;

// Re-export capability traits
pub use interface::{
    Audio, BatchGeneration, DocumentProcessing, Embeddings, Health, JsonMode, Metadata, Streaming,
    TokenCounting, ToolUse, Video, Vision,
};

// Re-export capability-specific types
pub use interface::{
    FinishReason, HealthStatus, ModelMetadata, StreamChunk, ToolDefinition, ToolResult,
};

// Re-export model implementations
#[cfg(feature = "gemini")]
pub use models::GeminiClient;

// Re-export narrative types
pub use narrative::{
    ActConfig, ActExecution, ExecutionFilter, ExecutionStatus, ExecutionSummary,
    InMemoryNarrativeRepository, Narrative, NarrativeError, NarrativeErrorKind,
    NarrativeExecution, NarrativeExecutor, NarrativeMetadata, NarrativeProvider,
    NarrativeRepository, NarrativeToc, VideoMetadata,
};

// Re-export rate limiting types
pub use rate_limit::{BoticelliConfig, ProviderConfig, Tier, TierConfig};

// Re-export provider-specific tier enums
#[cfg(feature = "gemini")]
pub use rate_limit::GeminiTier;
#[cfg(feature = "anthropic")]
pub use rate_limit::AnthropicTier;
pub use rate_limit::OpenAITier;
