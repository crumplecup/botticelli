//! Top-level error wrapper types.

use crate::{
    BackendError, BuilderError, ConfigError, GeminiError, HttpError, JsonError, NarrativeError,
    NotImplementedError, ServerError, StorageError,
};
#[cfg(feature = "database")]
use crate::DatabaseError;
#[cfg(feature = "tui")]
use crate::TuiError;

/// This is the foundation error enum. Additional variants will be added
/// by other botticelli crates during the workspace migration.
///
/// # Examples
///
/// ```
/// use botticelli_error::{BotticelliError, HttpError};
///
/// let http_err = HttpError::new("Connection failed");
/// let err: BotticelliError = http_err.into();
/// assert!(format!("{}", err).contains("HTTP Error"));
/// ```
#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum BotticelliErrorKind {
    /// HTTP error
    #[from(HttpError)]
    Http(HttpError),
    /// JSON serialization/deserialization error
    #[from(JsonError)]
    Json(JsonError),
    /// Generic backend error
    #[from(BackendError)]
    Backend(BackendError),
    /// Configuration error
    #[from(ConfigError)]
    Config(ConfigError),
    /// Builder error
    #[from(BuilderError)]
    Builder(BuilderError),
    /// Feature not yet implemented
    #[from(NotImplementedError)]
    NotImplemented(NotImplementedError),
    /// Storage error (Phase 3)
    #[from(StorageError)]
    Storage(StorageError),
    /// Gemini error (Phase 4)
    #[from(GeminiError)]
    Gemini(GeminiError),
    /// Database error (Phase 3.5)
    #[cfg(feature = "database")]
    #[from(DatabaseError)]
    Database(DatabaseError),
    /// Narrative error (Phase 3.5)
    #[from(NarrativeError)]
    Narrative(NarrativeError),
    /// TUI error (Phase 6)
    #[cfg(feature = "tui")]
    #[from(TuiError)]
    Tui(TuiError),
    /// Local inference server error
    #[from(ServerError)]
    Server(ServerError),
}

/// Botticelli error with kind discrimination.
///
/// # Examples
///
/// ```
/// use botticelli_error::{BotticelliError, BotticelliResult, ConfigError};
///
/// fn might_fail() -> BotticelliResult<()> {
///     Err(ConfigError::new("Missing field"))?
/// }
///
/// match might_fail() {
///     Ok(_) => println!("Success"),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("Botticelli Error: {}", _0)]
pub struct BotticelliError(Box<BotticelliErrorKind>);

impl BotticelliError {
    /// Create a new error from a kind.
    pub fn new(kind: BotticelliErrorKind) -> Self {
        Self(Box::new(kind))
    }

    /// Get the error kind.
    pub fn kind(&self) -> &BotticelliErrorKind {
        &self.0
    }
}

// Generic From implementation for any type that converts to BotticelliErrorKind
impl<T> From<T> for BotticelliError
where
    T: Into<BotticelliErrorKind>,
{
    fn from(err: T) -> Self {
        Self::new(err.into())
    }
}

/// Result type for Botticelli operations.
///
/// # Examples
///
/// ```
/// use botticelli_error::{BotticelliResult, HttpError};
///
/// fn fetch_data() -> BotticelliResult<String> {
///     Err(HttpError::new("404 Not Found"))?
/// }
/// ```
pub type BotticelliResult<T> = std::result::Result<T, BotticelliError>;
