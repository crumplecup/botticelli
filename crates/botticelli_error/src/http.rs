//! HTTP error types.

/// Specific HTTP error conditions.
#[derive(Debug, Clone, derive_more::Display)]
pub enum HttpErrorKind {
    /// Generic HTTP error with message
    #[display("HTTP error: {}", _0)]
    Message(String),

    /// Reqwest-specific error
    #[cfg(feature = "reqwest")]
    #[display("Reqwest error: {}", _0)]
    Reqwest(String),
}

/// HTTP error wrapping reqwest errors with source location.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("HTTP Error: {} at {}:{}", kind, file, line)]
pub struct HttpError {
    /// The error kind
    kind: HttpErrorKind,
    /// Line number where the error occurred
    line: u32,
    /// File where the error occurred
    file: &'static str,
}

impl HttpError {
    /// Create a new HttpError with the given kind at the current location.
    #[track_caller]
    pub fn new(kind: HttpErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

/// Support converting from String for convenience.
impl From<String> for HttpError {
    #[track_caller]
    fn from(message: String) -> Self {
        Self::new(HttpErrorKind::Message(message))
    }
}

/// Support converting from &str for convenience.
impl From<&str> for HttpError {
    #[track_caller]
    fn from(message: &str) -> Self {
        Self::new(HttpErrorKind::Message(message.to_string()))
    }
}

crate::impl_error_from_kind!(HttpErrorKind => HttpError);

#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for HttpError {
    #[track_caller]
    fn from(err: reqwest::Error) -> Self {
        Self::new(HttpErrorKind::Reqwest(format!("{:?}", err)))
    }
}

#[cfg(feature = "reqwest")]
crate::bridge_error!(reqwest::Error => HttpError => crate::BotticelliErrorKind);

