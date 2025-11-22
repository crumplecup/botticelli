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

mod http;
mod json;
mod config;
mod backend;
mod builder;
mod not_implemented;
mod storage;
mod gemini;
#[cfg(feature = "database")]
mod database;
mod narrative;
mod server;
#[cfg(feature = "tui")]
mod tui;
mod error;

pub use http::HttpError;
pub use json::JsonError;
pub use config::ConfigError;
pub use backend::BackendError;
pub use builder::{BuilderError, BuilderErrorKind};
pub use not_implemented::NotImplementedError;
pub use storage::{StorageError, StorageErrorKind};
pub use gemini::{GeminiError, GeminiErrorKind, RetryableError};
#[cfg(feature = "database")]
pub use database::{DatabaseError, DatabaseErrorKind};
pub use narrative::{NarrativeError, NarrativeErrorKind};
pub use server::{ServerError, ServerErrorKind};
#[cfg(feature = "tui")]
pub use tui::{TuiError, TuiErrorKind, TuiResult};
pub use error::{BotticelliError, BotticelliErrorKind, BotticelliResult};
