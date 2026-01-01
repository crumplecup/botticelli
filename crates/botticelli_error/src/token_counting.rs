//! Token counting errors.

/// Specific token counting error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum TokenCountingErrorKind {
    /// Failed to get tokenizer for model
    #[display("Failed to get tokenizer for model '{}': {}", model, message)]
    TokenizerNotFound {
        /// Model name that was requested
        model: String,
        /// Error message from tiktoken_rs
        message: String,
    },

    /// Invalid model name
    #[display("Invalid model name: {}", _0)]
    InvalidModel(String),
}

/// Token counting error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Token Counting Error: {} at {}:{}", kind, file, line)]
pub struct TokenCountingError {
    kind: TokenCountingErrorKind,
    line: u32,
    file: &'static str,
}

impl TokenCountingError {
    /// Create a new token counting error with caller location tracking.
    #[track_caller]
    pub fn new(kind: TokenCountingErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

crate::impl_error_from_kind!(TokenCountingErrorKind => TokenCountingError);

/// Result type for token counting operations.
pub type TokenCountingResult<T> = Result<T, TokenCountingError>;
