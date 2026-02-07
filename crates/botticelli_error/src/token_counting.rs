//! Token counting errors.

/// Specific token counting error conditions.
use rmcp::tool;

/// Specific token counting error conditions.
///
/// Categorizes errors from tokenization and token usage calculations.
#[derive(Debug, derive_more::Display)]
pub enum TokenCountingErrorKind {
    /// Failed to get tokenizer from tiktoken_rs
    #[display("Tiktoken error: {}", _0)]
    Tiktoken(Box<anyhow::Error>),

    /// Invalid model name
    #[display("Invalid model name: {}", _0)]
    InvalidModel(String),
}

/// Token counting error with location tracking.
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Token Counting Error: {} at {}:{}", kind, file, line)]
pub struct TokenCountingError {
    kind: TokenCountingErrorKind,
    line: u32,
    file: String,
}

impl TokenCountingError {
    /// Create a new token counting error with caller location tracking.
    #[tool]
    #[track_caller]
    pub fn new(kind: TokenCountingErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

crate::impl_error_from_kind!(TokenCountingErrorKind => TokenCountingError);

/// Result type for token counting operations.
pub type TokenCountingResult<T> = Result<T, TokenCountingError>;
