//! Error types for rate limiting operations.

use std::fmt;

/// Error kinds for rate limiting operations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RateLimitErrorKind {
    /// Configuration file error.
    Config(String),
    /// Rate limit exceeded.
    LimitExceeded(String),
    /// Invalid tier specification.
    InvalidTier(String),
}

impl fmt::Display for RateLimitErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RateLimitErrorKind::Config(msg) => write!(f, "Configuration error: {}", msg),
            RateLimitErrorKind::LimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
            RateLimitErrorKind::InvalidTier(msg) => write!(f, "Invalid tier: {}", msg),
        }
    }
}

/// Rate limiting error with location tracking.
#[derive(Debug, Clone)]
pub struct RateLimitError {
    kind: RateLimitErrorKind,
    line: u32,
    file: &'static str,
}

impl RateLimitError {
    /// Create a new rate limiting error with automatic location tracking.
    #[track_caller]
    pub fn new(kind: RateLimitErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }

    /// Get the error kind.
    pub fn kind(&self) -> &RateLimitErrorKind {
        &self.kind
    }
}

impl fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rate Limit Error: {} at line {} in {}",
            self.kind, self.line, self.file
        )
    }
}

impl std::error::Error for RateLimitError {}

impl<T> From<T> for RateLimitError
where
    T: Into<RateLimitErrorKind>,
{
    fn from(err: T) -> Self {
        Self::new(err.into())
    }
}
