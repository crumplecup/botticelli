//! OpenAI-compatible API errors.

use crate::BotticelliErrorKind;
use std::sync::Arc;

/// Specific error conditions for OpenAI-compatible APIs.
#[derive(Debug, Clone, derive_more::Display)]
pub enum OpenAIErrorKind {
    /// HTTP/network error
    #[display("HTTP error")]
    Http(Arc<reqwest::Error>),

    /// API returned an error
    #[display("API error (status {}): {}", status, message)]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Rate limit exceeded
    #[display("Rate limit exceeded")]
    RateLimit,

    /// Model not found
    #[display("Model not found: {}", _0)]
    ModelNotFound(String),

    /// Invalid request
    #[display("Invalid request: {}", _0)]
    InvalidRequest(String),

    /// Failed to parse response
    #[display("Response parsing failed")]
    ResponseParsing(Arc<serde_json::Error>),

    /// Builder error
    #[display("Builder error: {}", _0)]
    Builder(String),

    /// Environment variable error
    #[display("Environment variable error")]
    EnvVar(Arc<std::env::VarError>),
}

/// OpenAI-compatible API error with source location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("OpenAI Compat: {} at {}:{}", kind, file, line)]
pub struct OpenAIError {
    /// The specific error condition
    pub kind: OpenAIErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// Source file where error occurred
    pub file: &'static str,
}

impl OpenAIError {
    /// Creates a new OpenAI-compatible error with source location tracking.
    #[track_caller]
    pub fn new(kind: OpenAIErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

impl From<String> for OpenAIError {
    #[track_caller]
    fn from(message: String) -> Self {
        Self::new(OpenAIErrorKind::InvalidRequest(message))
    }
}

impl From<&str> for OpenAIError {
    #[track_caller]
    fn from(message: &str) -> Self {
        Self::new(OpenAIErrorKind::InvalidRequest(message.to_string()))
    }
}

// Bridge macros for automatic conversion chain
crate::impl_error_from_kind!(OpenAIErrorKind => OpenAIError);
crate::bridge_error!(OpenAIErrorKind => OpenAIError => BotticelliErrorKind);
// Note: reqwest::Error and serde_json::Error already have From impls via HttpError and JsonError

