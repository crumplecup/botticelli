//! Rate limiting error types.

/// Specific rate limiting error conditions.
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
    /// Builder validation error.
    #[display("Builder validation error: {_0}")]
    BuilderValidation(String),
    /// Builder failed with source error.
    #[display("Builder failed: {_0}")]
    BuilderFailed(String),
    /// Budget exceeded.
    #[display(
        "Budget exceeded: requested {requested_tokens} tokens, available: {available_tokens_minute} TPM, {available_tokens_day} TPD, {available_requests_minute} RPM, {available_requests_day} RPD"
    )]
    BudgetExceeded {
        /// Requested token count
        requested_tokens: u64,
        /// Available tokens per minute
        available_tokens_minute: u64,
        /// Available tokens per day
        available_tokens_day: u64,
        /// Available requests per minute
        available_requests_minute: u64,
        /// Available requests per day
        available_requests_day: u64,
    },
}

/// Rate limiting error with location tracking.
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Rate Limit Error: {} at line {} in {}", kind, line, file)]
pub struct RateLimitError {
    /// The kind of error that occurred
    kind: RateLimitErrorKind,
    /// Line number where error was created
    line: u32,
    /// File where error was created
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
}

// Conversion from String (derive_builder errors are String-based)
impl From<String> for RateLimitErrorKind {
    fn from(msg: String) -> Self {
        Self::BuilderFailed(msg)
    }
}

// ErrorKind → Error conversion
crate::impl_error_from_kind!(RateLimitErrorKind => RateLimitError);

// String → RateLimitError (via ErrorKind)
impl From<String> for RateLimitError {
    #[track_caller]
    fn from(msg: String) -> Self {
        Self::new(RateLimitErrorKind::from(msg))
    }
}

// TierConfigBuilderError → RateLimitErrorKind (for wrapping)
impl From<derive_builder::UninitializedFieldError> for RateLimitErrorKind {
    fn from(err: derive_builder::UninitializedFieldError) -> Self {
        Self::BuilderFailed(err.to_string())
    }
}

// TierConfigBuilderError → RateLimitError (via ErrorKind)
impl From<derive_builder::UninitializedFieldError> for RateLimitError {
    #[track_caller]
    fn from(err: derive_builder::UninitializedFieldError) -> Self {
        RateLimitErrorKind::from(err).into()
    }
}

// Bridge to umbrella error (generates BotticelliErrorKind and BotticelliError conversions)
crate::bridge_error!(derive_builder::UninitializedFieldError => RateLimitError => crate::BotticelliErrorKind);

// Bridge String → RateLimitError → BotticelliErrorKind
crate::bridge_error!(String => RateLimitError => crate::BotticelliErrorKind);
