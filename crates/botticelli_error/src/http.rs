//! HTTP error types.

/// HTTP error wrapping reqwest errors with source location.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("HTTP Error: {} at {}:{}", message, file, line)]
pub struct HttpError {
    /// The underlying error message
    message: String,
    /// Line number where the error occurred
    line: u32,
    /// File where the error occurred
    file: &'static str,
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
    /// assert!(err.message().contains("Connection refused"));
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
impl From<String> for HttpError {
    #[track_caller]
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

/// Support converting from &str for convenience.
impl From<&str> for HttpError {
    #[track_caller]
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

// Add support for wrapping external errors when they're available
#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for HttpError {
    #[track_caller]
    fn from(err: reqwest::Error) -> Self {
        Self::new(err.to_string())
    }
}

// Bridge reqwest::Error to BotticelliErrorKind
#[cfg(feature = "reqwest")]
crate::bridge_error!(reqwest::Error => HttpError => crate::BotticelliErrorKind);

