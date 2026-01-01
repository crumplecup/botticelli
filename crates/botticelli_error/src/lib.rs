//! Error types for the Botticelli library.
//!
//! This crate provides the foundation error types used throughout the Botticelli ecosystem.
//!
//! # Error Hierarchy
//!
//! All errors follow the `ErrorKind` + wrapper struct pattern for clean error handling:
//! - `*ErrorKind` enum defines specific error conditions
//! - `*Error` struct wraps the kind with source location tracking
//! - All errors use `#[track_caller]` for automatic location capture
//!
//! # Examples
//!
//! ```
//! use botticelli_error::{BotticelliResult, HttpError};
//!
//! fn fetch_data() -> BotticelliResult<String> {
//!     Err(HttpError::from("Connection refused"))?
//! }
//!
//! match fetch_data() {
//!     Ok(data) => println!("Got: {}", data),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod macros;

mod backend;
mod builder;
mod chat;
mod config;
#[cfg(feature = "database")]
mod database;
mod error;
mod gemini;
mod http;
mod io;
mod json;
mod mcp;
#[cfg(feature = "models")]
mod models;
mod narrative;
mod not_implemented;
mod observability;
mod provider;
mod server;
mod storage;
mod token_counting;
#[cfg(feature = "tui")]
mod tui;

pub use backend::BackendError;
pub use builder::{BuilderError, BuilderErrorKind};
pub use chat::{ChatError, ChatErrorKind, ChatResult};
pub use config::ConfigError;
#[cfg(feature = "database")]
pub use database::{DatabaseError, DatabaseErrorKind, DieselConnectionError, DieselError};
#[cfg(feature = "serde_json")]
pub use database::SerdeJsonError;
pub use error::{BotticelliError, BotticelliErrorKind, BotticelliResult};
pub use gemini::{GeminiError, GeminiErrorKind};
pub use http::{HttpError, HttpErrorKind};
pub use io::IoError;
pub use json::{JsonError, JsonErrorKind};
#[cfg(feature = "serde_json")]
pub use json::SerdeJsonError as JsonSerdeJsonError;
pub use mcp::{McpError, McpErrorKind, McpResult};
#[cfg(feature = "anthropic")]
pub use models::AnthropicErrorKind;
#[cfg(feature = "huggingface")]
pub use models::HuggingFaceErrorKind;

#[cfg(feature = "groq")]
pub use models::GroqErrorKind;
#[cfg(feature = "ollama")]
pub use models::OllamaErrorKind;
#[cfg(feature = "models")]
pub use models::{ModelsError, ModelsErrorKind, ModelsResult};
pub use narrative::{NarrativeError, NarrativeErrorKind};
pub use not_implemented::NotImplementedError;
pub use observability::{ObservabilityError, ObservabilityErrorKind, ObservabilityResult};
pub use provider::{ProviderError, ProviderErrorKind, ProviderResult};
#[cfg(feature = "reqwest")]
pub use provider::ProviderReqwestError;
#[cfg(feature = "serde_json")]
pub use provider::ProviderSerdeJsonError;
pub use server::{ServerError, ServerErrorKind};
pub use storage::{StorageError, StorageErrorKind};
pub use token_counting::{TokenCountingError, TokenCountingErrorKind, TokenCountingResult};
#[cfg(feature = "tui")]
pub use tui::{TuiError, TuiErrorKind, TuiIoError, TuiResult};
