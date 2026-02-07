//! Configuration error types.

/// Configuration error with source location.
use rmcp::tool;

/// Configuration error with source location.
///
/// Wraps configuration parsing and validation errors with location tracking.
#[derive(
    Debug,
    Clone,
    derive_more::Display,
    derive_more::Error,
    derive_getters::Getters,
    elicitation::Elicit,
)]
#[display("Configuration Error: {} at line {} in {}", message, line, file)]
pub struct ConfigError {
    /// Error message
    message: String,
    /// Line number where the error occurred
    line: u32,
    /// File where the error occurred
    file: String,
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
    /// assert!(err.message().contains("Missing required"));
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
