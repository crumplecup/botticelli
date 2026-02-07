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

// Re-export rmcp::tool when mcp feature is enabled
pub use rmcp::tool;

#[cfg(feature = "models")]
extern crate gemini_rust;

mod macros;

mod backend;
mod builder;
mod chat;
mod config;
#[cfg(feature = "database")]
mod database;
mod discord;
mod env;
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
mod openai;
mod provider;
mod rate_limit;
mod security;
mod server;
mod social;
mod storage;
mod token_counting;
mod toml;
#[cfg(feature = "tui")]
mod tui;
mod validation;

pub use backend::BackendError;
pub use builder::{BuilderError, BuilderErrorKind};
pub use chat::{ChatError, ChatErrorKind, ChatResult, SamplingError, SamplingErrorKind};
pub use config::ConfigError;
#[cfg(feature = "database")]
pub use database::{DatabaseError, DatabaseErrorKind, DieselConnectionError, DieselError};
pub use discord::{
    DiscordError, DiscordErrorKind, DiscordErrorResult, DiscordErrorSeverity, DiscordResult,
};
pub use env::{EnvError, EnvErrorKind};
pub use error::{BotticelliError, BotticelliErrorKind, BotticelliResult};
pub use gemini::{GeminiError, GeminiErrorKind};
pub use http::{HttpError, HttpErrorKind};
pub use io::IoError;
pub use json::SerdeJsonError as JsonSerdeJsonError;
pub use json::{JsonError, JsonErrorKind};
pub use mcp::SerdeJsonError as McpSerdeJsonError;
pub use mcp::{McpError, McpErrorKind, McpResult};
#[cfg(feature = "anthropic")]
pub use models::{AnthropicError, AnthropicErrorKind};
#[cfg(feature = "models")]
pub use models::{ModelsError, ModelsErrorKind, ModelsResult};
#[cfg(feature = "ollama")]
pub use models::{OllamaError, OllamaErrorKind, OllamaResult};
pub use narrative::{NarrativeError, NarrativeErrorKind, NarrativeResult};
pub use not_implemented::NotImplementedError;
pub use observability::{ObservabilityError, ObservabilityErrorKind, ObservabilityResult};
pub use openai::{OpenAIError, OpenAIErrorKind};
#[cfg(feature = "reqwest")]
pub use provider::ProviderReqwestError;
pub use provider::ProviderSerdeJsonError;
pub use provider::{ProviderError, ProviderErrorKind, ProviderResult};
pub use rate_limit::{RateLimitError, RateLimitErrorKind};
pub use security::{SecurityError, SecurityErrorKind, SecurityResult};
pub use server::{ServerError, ServerErrorKind};
pub use social::{BotCommandError, BotCommandErrorKind, BotCommandResult};
pub use storage::{StorageError, StorageErrorKind};
pub use token_counting::{TokenCountingError, TokenCountingErrorKind, TokenCountingResult};
pub use toml::TomlError;
#[cfg(feature = "tui")]
pub use tui::{TuiError, TuiErrorKind, TuiIoError, TuiResult};
pub use validation::{
    ValidationError, ValidationErrorKind, ValidationLocation, ValidationResult, ValidationWarning,
    ValidationWarningKind,
};
