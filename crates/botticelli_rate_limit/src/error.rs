//! Error types for rate limiting operations.

/// Error kinds for rate limiting operations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
pub enum RateLimitErrorKind {
    /// Configuration file error.
    #[display("Configuration error: {_0}")]
    Config(String),
    /// Rate limit exceeded.
    #[display("Rate limit exceeded: {_0}")]
    LimitExceeded(String),
    /// Invalid tier specification.
    #[display("Invalid tier: {_0}")]
    InvalidTier(String),
}

/// Rate limiting error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Rate Limit Error: {} at line {} in {}", kind, line, file)]
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

impl<T> From<T> for RateLimitError
where
    T: Into<RateLimitErrorKind>,
{
    fn from(err: T) -> Self {
        Self::new(err.into())
    }
}
