//! JSON error types.

/// JSON serialization/deserialization error with source location.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("JSON Error: {} at {}:{}", message, file, line)]
pub struct JsonError {
    /// The underlying error message
    message: String,
    /// Line number where the error occurred
    line: u32,
    /// File where the error occurred
    file: &'static str,
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
    /// assert!(err.message().contains("Invalid JSON"));
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

/// Support converting from String for convenience.
impl From<String> for JsonError {
    #[track_caller]
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

/// Support converting from &str for convenience.
impl From<&str> for JsonError {
    #[track_caller]
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

// Add support for wrapping serde_json errors when feature is enabled
#[cfg(feature = "serde_json")]
impl From<serde_json::Error> for JsonError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        Self::new(err.to_string())
    }
}

// Bridge serde_json::Error to BotticelliErrorKind
#[cfg(feature = "serde_json")]
crate::bridge_error!(serde_json::Error => JsonError => crate::BotticelliErrorKind);

