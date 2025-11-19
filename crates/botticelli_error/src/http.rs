//! HTTP error types.

/// HTTP error wrapping reqwest errors with source location.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("HTTP Error: {} at line {} in {}", message, line, file)]
pub struct HttpError {
    /// The underlying error message
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl HttpError {
    /// Create a new HttpError with the given message at the current location.
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_error::HttpError;
    ///
    /// let err = HttpError::new("Connection refused");
    /// assert!(err.message.contains("Connection refused"));
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
