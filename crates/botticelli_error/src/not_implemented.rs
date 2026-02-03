//! Not implemented error types.

/// Not implemented error with source location.
#[cfg(feature = "mcp")]
use crate::tool;

#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Not Implemented: {} at line {} in {}", message, line, file)]
pub struct NotImplementedError {
    /// Description of what is not implemented
    message: String,
    /// Line number where the error occurred
    line: u32,
    /// File where the error occurred
    file: String,
}

impl NotImplementedError {
    /// Create a new NotImplementedError with the given message at the current location.
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_error::NotImplementedError;
    ///
    /// let err = NotImplementedError::new("Feature X not yet supported");
    /// assert!(err.message().contains("not yet supported"));
    /// ```
    #[tool]
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

/// Support converting from String for convenience.
impl From<String> for NotImplementedError {
    #[track_caller]
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

/// Support converting from &str for convenience.
impl From<&str> for NotImplementedError {
    #[track_caller]
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}
