//! Not implemented error types.

/// Not implemented error with source location.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Not Implemented: {} at line {} in {}", message, line, file)]
pub struct NotImplementedError {
    /// Description of what is not implemented
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
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
    /// assert!(err.message.contains("not yet supported"));
    /// ```
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            line: location.line(),
            file: location.file(),
        }
    }
}
