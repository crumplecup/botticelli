//! Generic inference server traits and OpenAI-compatible client for Botticelli
//!
//! This crate provides trait interfaces for local LLM inference servers and a generic
//! client for interacting with OpenAI-compatible APIs. Server implementations are
//! provided by external crates (e.g., `botticelli_mistral` for MistralRS).
//!
//! # Features
//!
//! - **Trait Interfaces**: `InferenceServer`, `ServerLauncher`, `ModelManager`
//! - **Generic Client**: OpenAI-compatible HTTP client with streaming support
//! - **Request/Response Types**: Standard chat completion API types
//! - **Full Observability**: Comprehensive tracing instrumentation
//!
//! # Example with External Implementation
//!
//! ```rust,no_run
//! use botticelli_server::{ServerClient, ServerConfig};
//! // Server implementation from external crate:
//! // use botticelli_mistral::{MistralLauncher, MistralConfig, MistralModelManager, MistralModelSpec};
//! // use botticelli_server::{InferenceServer, ServerLauncher, ModelManagerTrait};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Assuming server is already running at localhost:8080
//!     let config = ServerConfig::new("http://localhost:8080", "mistral-7b");
//!     let client = ServerClient::new(config);
//!
//!     // Use client to interact with the server...
//!
//!     Ok(())
//! }
//! ```
//!
//! # Server Implementations
//!
//! Server implementations live in external crates to avoid git dependencies:
//!
//! - **`botticelli_mistral`**: MistralRS GGUF model inference
//! - **`botticelli_llamacpp`**: llama.cpp integration (community)
//! - **`botticelli_ollama`**: Ollama integration (community)
//!
//! See trait documentation for implementation guidelines.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod actor_traits;
mod bots;
mod client;
mod config;
mod convert;
mod metrics;
mod observability;
mod request;
mod response;
mod schedule;
mod traits;

pub use actor_traits::{
    ActorManager, ActorServer, ActorServerResult, ContentPoster, StatePersistence, TaskScheduler,
};
pub use bots::{
    BotServer, CurationBot, CurationBotArgs, CurationMessage, GenerationBot, GenerationBotArgs,
    GenerationMessage, PostingBot, PostingBotArgs, PostingMessage,
};
pub use botticelli_error::{ServerError, ServerErrorKind};
pub use client::ServerClient;
pub use config::ServerConfig;
pub use metrics::{BotMetrics, NarrativeMetrics, PipelineMetrics, ServerMetrics};
pub use observability::{init_observability, shutdown_observability, ObservabilityConfig};
pub use request::{ChatCompletionRequest, Message};
pub use response::{
    ChatCompletionChunk, ChatCompletionResponse, Choice, ChoiceMessage, ChunkChoice, Delta, Usage,
};
pub use schedule::{Schedule, ScheduleCheck, ScheduleType};
pub use traits::{InferenceServer, ModelManager as ModelManagerTrait, ServerLauncher};
