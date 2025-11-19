//! Configuration error types.

/// Configuration error with source location.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Configuration Error: {} at line {} in {}", message, line, file)]
pub struct ConfigError {
    /// Error message
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl ConfigError {
    /// Create a new ConfigError with the given message at the current location.
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_error::ConfigError;
    ///
    /// let err = ConfigError::new("Missing required field");
    /// assert!(err.message.contains("Missing required"));
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
