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
//!     Err(HttpError::new("Connection refused"))?
//! }
//!
//! match fetch_data() {
//!     Ok(data) => println!("Got: {}", data),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod backend;
mod builder;
mod config;
#[cfg(feature = "database")]
mod database;
mod error;
mod gemini;
mod http;
mod json;
#[cfg(feature = "models")]
mod models;
mod narrative;
mod not_implemented;
mod server;
mod storage;
#[cfg(feature = "tui")]
mod tui;

pub use backend::BackendError;
pub use builder::{BuilderError, BuilderErrorKind};
pub use config::ConfigError;
#[cfg(feature = "database")]
pub use database::{DatabaseError, DatabaseErrorKind};
pub use error::{BotticelliError, BotticelliErrorKind, BotticelliResult};
pub use gemini::{GeminiError, GeminiErrorKind, RetryableError};
pub use http::HttpError;
pub use json::JsonError;
#[cfg(feature = "ollama")]
pub use models::OllamaErrorKind;
#[cfg(feature = "models")]
pub use models::{ModelsError, ModelsErrorKind, ModelsResult};
pub use narrative::{NarrativeError, NarrativeErrorKind};
pub use not_implemented::NotImplementedError;
pub use server::{ServerError, ServerErrorKind};
pub use storage::{StorageError, StorageErrorKind};
#[cfg(feature = "tui")]
pub use tui::{TuiError, TuiErrorKind, TuiResult};
