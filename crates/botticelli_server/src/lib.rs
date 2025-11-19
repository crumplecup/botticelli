//! Local inference server integration for Botticelli
//!
//! This crate provides a client for interacting with local inference servers
//! (like mistral.rs), enabling fast local model inference for large language models.
//!
//! # Features
//!
//! - OpenAI-compatible API client
//! - Streaming and non-streaming inference
//! - Full observability with tracing instrumentation
//!
//! # Setup
//!
//! 1. Install an inference server (e.g., mistral.rs)
//! 2. Start the server: `mistralrs_server --port 8080`
//! 3. Set environment variables:
//!    - `INFERENCE_SERVER_BASE_URL` (default: "http://localhost:8080")
//!    - `INFERENCE_SERVER_MODEL` (required, e.g., "microsoft/Phi-3.5-mini-instruct")
//!
//! # Example
//!
//! ```rust,no_run
//! use botticelli_server::{ServerClient, ServerConfig, Message, ChatCompletionRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ServerConfig::new("http://localhost:8080", "phi-3.5");
//!     let client = ServerClient::new(config);
//!
//!     let request = ChatCompletionRequest {
//!         model: "phi-3.5".into(),
//!         messages: vec![Message::user("Explain Rust ownership")],
//!         max_tokens: Some(100),
//!         temperature: Some(0.7),
//!         top_p: None,
//!         stream: None,
//!     };
//!
//!     let response = client.chat_completion(request).await?;
//!     println!("{}", response.choices[0].message.content);
//!
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod client;
mod config;
mod convert;
mod request;
mod response;

pub use botticelli_error::{ServerError, ServerErrorKind};
pub use client::ServerClient;
pub use config::ServerConfig;
pub use request::{ChatCompletionRequest, Message};
pub use response::{
    ChatCompletionChunk, ChatCompletionResponse, Choice, ChoiceMessage, ChunkChoice, Delta, Usage,
};
