//! Error types for the Boticelli library.

#[cfg(feature = "database")]
use crate::DatabaseError;
#[cfg(feature = "discord")]
use crate::DiscordError;
#[cfg(feature = "gemini")]
use crate::GeminiError;
#[cfg(feature = "tui")]
use crate::TuiError;
use crate::{NarrativeError, StorageError};

/// HTTP error wrapping reqwest errors with source location.
#[derive(Debug)]
pub struct HttpError {
    /// The underlying reqwest error
    pub error: reqwest::Error,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl HttpError {
    /// Create a new HttpError with the given reqwest error at the current location.
    #[track_caller]
    pub fn new(error: reqwest::Error) -> Self {
        let location = std::panic::Location::caller();
        Self {
            error,
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HTTP Error: {} at line {} in {}",
            self.error, self.line, self.file
        )
    }
}

impl std::error::Error for HttpError {}

/// JSON serialization/deserialization error with source location.
#[derive(Debug)]
pub struct JsonError {
    /// The underlying serde_json error
    pub error: serde_json::Error,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl JsonError {
    /// Create a new JsonError with the given serde_json error at the current location.
    #[track_caller]
    pub fn new(error: serde_json::Error) -> Self {
        let location = std::panic::Location::caller();
        Self {
            error,
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JSON Error: {} at line {} in {}",
            self.error, self.line, self.file
        )
    }
}

impl std::error::Error for JsonError {}

/// Configuration error with source location.
#[derive(Debug)]
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

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Configuration Error: {} at line {} in {}",
            self.message, self.line, self.file
        )
    }
}

impl std::error::Error for ConfigError {}

/// Not implemented error with source location.
#[derive(Debug)]
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

impl std::fmt::Display for NotImplementedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Not Implemented: {} at line {} in {}",
            self.message, self.line, self.file
        )
    }
}

impl std::error::Error for NotImplementedError {}

/// Backend error with source location.
#[derive(Debug)]
pub struct BackendError {
    /// Error message
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl BackendError {
    /// Create a new BackendError with the given message at the current location.
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

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Backend Error: {} at line {} in {}",
            self.message, self.line, self.file
        )
    }
}

impl std::error::Error for BackendError {}

/// Crate-level error variants.
#[derive(Debug, derive_more::From)]
pub enum BoticelliErrorKind {
    /// HTTP error from reqwest
    #[from(HttpError)]
    Http(HttpError),
    /// JSON serialization/deserialization error
    #[from(JsonError)]
    Json(JsonError),
    /// Generic backend error
    #[from(BackendError)]
    Backend(BackendError),
    /// Gemini-specific error
    #[cfg(feature = "gemini")]
    #[from(GeminiError)]
    Gemini(GeminiError),
    /// Database error
    #[cfg(feature = "database")]
    #[from(DatabaseError)]
    Database(DatabaseError),
    /// Discord integration error
    #[cfg(feature = "discord")]
    #[from(DiscordError)]
    Discord(DiscordError),
    /// Narrative error
    #[from(NarrativeError)]
    Narrative(NarrativeError),
    /// Configuration error
    #[from(ConfigError)]
    Config(ConfigError),
    /// Feature not yet implemented
    #[from(NotImplementedError)]
    NotImplemented(NotImplementedError),
    /// Storage error
    #[from(StorageError)]
    Storage(StorageError),
    /// TUI error
    #[cfg(feature = "tui")]
    #[from(TuiError)]
    Tui(TuiError),
}

impl std::fmt::Display for BoticelliErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoticelliErrorKind::Http(e) => write!(f, "{}", e),
            BoticelliErrorKind::Json(e) => write!(f, "{}", e),
            BoticelliErrorKind::Backend(e) => write!(f, "{}", e),
            #[cfg(feature = "gemini")]
            BoticelliErrorKind::Gemini(e) => write!(f, "{}", e),
            #[cfg(feature = "database")]
            BoticelliErrorKind::Database(e) => write!(f, "{}", e),
            #[cfg(feature = "discord")]
            BoticelliErrorKind::Discord(e) => write!(f, "{}", e),
            BoticelliErrorKind::Narrative(e) => write!(f, "{}", e),
            BoticelliErrorKind::Config(e) => write!(f, "{}", e),
            BoticelliErrorKind::NotImplemented(e) => write!(f, "{}", e),
            BoticelliErrorKind::Storage(e) => write!(f, "{}", e),
            #[cfg(feature = "tui")]
            BoticelliErrorKind::Tui(e) => write!(f, "{}", e),
        }
    }
}

/// Boticelli error with kind discrimination.
#[derive(Debug)]
pub struct BoticelliError(Box<BoticelliErrorKind>);

impl BoticelliError {
    /// Create a new error from a kind.
    pub fn new(kind: BoticelliErrorKind) -> Self {
        Self(Box::new(kind))
    }

    /// Get the error kind.
    pub fn kind(&self) -> &BoticelliErrorKind {
        &self.0
    }
}

impl std::fmt::Display for BoticelliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Boticelli Error: {}", self.0)
    }
}

impl std::error::Error for BoticelliError {}

// Generic From implementation for any type that converts to BoticelliErrorKind
impl<T> From<T> for BoticelliError
where
    T: Into<BoticelliErrorKind>,
{
    fn from(err: T) -> Self {
        Self::new(err.into())
    }
}

#[cfg(feature = "database")]
impl From<diesel::result::Error> for BoticelliError {
    fn from(err: diesel::result::Error) -> Self {
        BackendError::new(format!("Database error: {}", err)).into()
    }
}

/// Result type for Boticelli operations.
pub type BoticelliResult<T> = std::result::Result<T, BoticelliError>;
