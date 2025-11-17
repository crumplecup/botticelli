//! Google Gemini API client implementation.
//!
//! This module provides two clients for the Gemini API:
//! - [`GeminiClient`] - REST API client for standard requests
//! - [`GeminiLiveClient`] - WebSocket client for Live API (bidirectional streaming)
//!
//! # REST API Client
//!
//! The REST API client supports:
//! - Per-request model selection
//! - Client pooling with lazy initialization
//! - Per-model rate limiting
//! - Thread-safe concurrent access
//! - SSE streaming (via `Streaming` trait)
//!
//! # Live API Client
//!
//! The Live API client supports:
//! - WebSocket bidirectional streaming
//! - Real-time audio/video interactions
//! - Text chat
//! - Better rate limits on free tier
//!
//! # Example
//!
//! ```no_run
//! use boticelli::{BoticelliDriver, GeminiClient, GenerateRequest, Message, Role, Input};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // REST API client
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
//! ```

mod error;
mod client;
pub mod live_protocol;
mod live_client;
mod live_rate_limit;

pub use error::{GeminiError, GeminiErrorKind};
pub use client::{GeminiClient, TieredGemini};
pub use live_client::{GeminiLiveClient, LiveSession};
pub use live_protocol::GenerationConfig;
pub use live_rate_limit::LiveRateLimiter;
