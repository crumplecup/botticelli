//! Backend error types.

/// Backend error with source location.
#[cfg(feature = "mcp")]
use crate::tool;

#[derive(Debug, Clone, derive_more::Display, derive_more::Error, elicitation::Elicit)]
#[display("Backend Error: {} at line {} in {}", message, line, file)]
pub struct BackendError {
    /// Error message
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: String,
}

impl BackendError {
    /// Create a new BackendError with the given message at the current location.
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_error::BackendError;
    ///
    /// let err = BackendError::new("Backend service unavailable");
    /// assert!(err.message.contains("unavailable"));
    /// ```
    #[cfg_attr(feature = "mcp", tool)]
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
