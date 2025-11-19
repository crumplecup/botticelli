//! JSON error types.

/// JSON serialization/deserialization error with source location.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("JSON Error: {} at line {} in {}", message, line, file)]
pub struct JsonError {
    /// The underlying error message
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl JsonError {
    /// Create a new JsonError with the given message at the current location.
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_error::JsonError;
    ///
    /// let err = JsonError::new("Invalid JSON syntax");
    /// assert!(err.message.contains("Invalid JSON"));
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
