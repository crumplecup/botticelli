//! Provider error types.

/// Reqwest error with source tracking for provider operations.
#[cfg(feature = "mcp")]
use crate::tool;

#[cfg(feature = "reqwest")]
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Reqwest error: {:?} at {}:{}", source, file, line)]
pub struct ProviderReqwestError {
    /// The reqwest error source
    source: Box<reqwest::Error>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

#[cfg(feature = "reqwest")]
impl ProviderReqwestError {
    /// Create a new ProviderReqwestError with automatic location tracking.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(err: reqwest::Error) -> Self {
        let location = std::panic::Location::caller();
        Self {
            source: Box::new(err),
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

/// Serde JSON error with source tracking for provider operations.
#[cfg(feature = "serde_json")]
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Serde JSON error: {:?} at {}:{}", source, file, line)]
pub struct ProviderSerdeJsonError {
    /// The serde_json error source
    source: Box<serde_json::Error>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

#[cfg(feature = "serde_json")]
impl ProviderSerdeJsonError {
    /// Create a new ProviderSerdeJsonError with automatic location tracking.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(err: serde_json::Error) -> Self {
        let location = std::panic::Location::caller();
        Self {
            source: Box::new(err),
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

#[cfg(feature = "serde_json")]
impl Clone for ProviderSerdeJsonError {
    fn clone(&self) -> Self {
        // serde_json::Error is not Clone, reconstruct from message
        Self {
            source: Box::new(serde_json::Error::io(std::io::Error::other(format!(
                "{:?}",
                self.source
            )))),
            line: self.line,
            file: self.file.clone(),
        }
    }
}

/// Errors from provider operations.
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Provider {}: {} at {}:{}", provider, kind, file, line)]
pub struct ProviderError {
    /// Provider name
    provider: String,
    /// Error kind
    kind: ProviderErrorKind,
    /// Line number where error occurred
    line: u32,
    /// File where error occurred
    file: String,
}

/// Types of provider errors.
#[derive(Debug, derive_more::Display)]
pub enum ProviderErrorKind {
    /// API error from the provider.
    #[display("API error: {}", _0)]
    ApiError(String),

    /// Reqwest error
    #[cfg(feature = "reqwest")]
    #[display("{}", _0)]
    Reqwest(ProviderReqwestError),

    /// Authentication failed.
    #[display("Authentication failed")]
    AuthenticationFailed,

    /// Rate limit exceeded.
    #[display("Rate limit exceeded")]
    RateLimitExceeded,

    /// Invalid request.
    #[display("Invalid request: {}", _0)]
    InvalidRequest(String),

    /// Response parsing failed.
    #[display("Response parsing failed: {}", _0)]
    ParsingError(String),

    /// Serde JSON parsing error
    #[cfg(feature = "serde_json")]
    #[display("{}", _0)]
    SerdeJson(ProviderSerdeJsonError),
}

impl ProviderError {
    /// Create a new provider error with location tracking.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(provider: impl Into<String>, kind: ProviderErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            provider: provider.into(),
            kind,
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }
}

#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for ProviderError {
    #[track_caller]
    fn from(err: reqwest::Error) -> Self {
        Self::new(
            "http",
            ProviderErrorKind::Reqwest(ProviderReqwestError::new(err)),
        )
    }
}

#[cfg(feature = "serde_json")]
impl From<serde_json::Error> for ProviderError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        Self::new(
            "json",
            ProviderErrorKind::SerdeJson(ProviderSerdeJsonError::new(err)),
        )
    }
}

/// Result type for provider operations.
pub type ProviderResult<T> = std::result::Result<T, ProviderError>;
