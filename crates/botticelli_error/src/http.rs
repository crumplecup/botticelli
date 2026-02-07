//! HTTP error types.
use serde::{Deserialize, Serialize};

/// Specific HTTP error conditions.
#[cfg(feature = "mcp")]
use crate::tool;

use elicitation::{Prompt, Select};

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Display, schemars::JsonSchema, elicitation::Elicit)]
pub enum HttpErrorKind {
    /// Generic HTTP error with message
    #[display("HTTP error: {}", _0)]
    Message(String),

    /// Invalid header value
    #[cfg(feature = "reqwest")]
    #[display("Invalid header value: {}", _0)]
    InvalidHeaderValue(String),

    /// Reqwest-specific error
    #[cfg(feature = "reqwest")]
    #[display("Reqwest error: {}", _0)]
    Reqwest(String),
}

/// HTTP error wrapping reqwest errors with source location.
#[derive(
    Debug,
    Clone,
    derive_more::Display,
    derive_more::Error,
    derive_getters::Getters,
    elicitation::Elicit,
)]
#[display("HTTP Error: {} at {}:{}", kind, file, line)]
pub struct HttpError {
    /// The error kind
    kind: HttpErrorKind,
    /// Line number where the error occurred
    line: u32,
    /// File where the error occurred
    file: String,
}

impl HttpError {
    /// Create a new HttpError with the given kind at the current location.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(kind: HttpErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

/// Support converting from String for convenience.
impl From<String> for HttpError {
    #[track_caller]
    fn from(message: String) -> Self {
        Self::new(HttpErrorKind::Message(message))
    }
}

/// Support converting from &str for convenience.
impl From<&str> for HttpError {
    #[track_caller]
    fn from(message: &str) -> Self {
        Self::new(HttpErrorKind::Message(message.to_string()))
    }
}

crate::impl_error_from_kind!(HttpErrorKind => HttpError);

#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for HttpError {
    #[track_caller]
    fn from(err: reqwest::Error) -> Self {
        Self::new(HttpErrorKind::Reqwest(format!("{:?}", err)))
    }
}

#[cfg(feature = "reqwest")]
impl From<reqwest::header::InvalidHeaderValue> for HttpError {
    #[track_caller]
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Self::new(HttpErrorKind::InvalidHeaderValue(err.to_string()))
    }
}

#[cfg(feature = "reqwest")]
crate::bridge_error!(reqwest::Error => HttpError => crate::BotticelliErrorKind);

#[cfg(feature = "reqwest")]
crate::bridge_error!(reqwest::header::InvalidHeaderValue => HttpError => crate::BotticelliErrorKind);
