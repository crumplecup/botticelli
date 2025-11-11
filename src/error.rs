//! Error types for the Boticelli library.

/// Crate-level error variants.
#[derive(Debug)]
pub enum BoticelliErrorKind {
    /// HTTP error from reqwest
    Http(reqwest::Error),
    /// JSON serialization/deserialization error
    Json(serde_json::Error),
    /// Generic backend error (deprecated - use specific error types)
    Backend(String),
    /// Gemini-specific error
    #[cfg(feature = "gemini")]
    Gemini(crate::GeminiError),
    /// Database error
    #[cfg(feature = "database")]
    Database(crate::DatabaseError),
    /// Narrative error
    Narrative(crate::NarrativeError),
    /// Configuration error
    Config(String),
    /// Feature not yet implemented
    NotImplemented(String),
}

// Manual From implementations to avoid conflicts with multiple String variants
impl From<reqwest::Error> for BoticelliErrorKind {
    fn from(err: reqwest::Error) -> Self {
        BoticelliErrorKind::Http(err)
    }
}

impl From<serde_json::Error> for BoticelliErrorKind {
    fn from(err: serde_json::Error) -> Self {
        BoticelliErrorKind::Json(err)
    }
}

#[cfg(feature = "gemini")]
impl From<crate::models::gemini::GeminiError> for BoticelliErrorKind {
    fn from(err: crate::models::gemini::GeminiError) -> Self {
        BoticelliErrorKind::Gemini(err)
    }
}

#[cfg(feature = "database")]
impl From<crate::DatabaseError> for BoticelliErrorKind {
    fn from(err: crate::DatabaseError) -> Self {
        BoticelliErrorKind::Database(err)
    }
}

impl From<crate::narrative::NarrativeError> for BoticelliErrorKind {
    fn from(err: crate::narrative::NarrativeError) -> Self {
        BoticelliErrorKind::Narrative(err)
    }
}

impl std::fmt::Display for BoticelliErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoticelliErrorKind::Http(e) => write!(f, "{}", e),
            BoticelliErrorKind::Json(e) => write!(f, "{}", e),
            BoticelliErrorKind::Backend(msg) => write!(f, "{}", msg),
            #[cfg(feature = "gemini")]
            BoticelliErrorKind::Gemini(e) => write!(f, "{}", e),
            #[cfg(feature = "database")]
            BoticelliErrorKind::Database(e) => write!(f, "{}", e),
            BoticelliErrorKind::Narrative(e) => write!(f, "{}", e),
            BoticelliErrorKind::Config(msg) => write!(f, "Configuration error: {}", msg),
            BoticelliErrorKind::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
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

impl From<BoticelliErrorKind> for BoticelliError {
    fn from(kind: BoticelliErrorKind) -> Self {
        Self::new(kind)
    }
}

impl From<reqwest::Error> for BoticelliError {
    fn from(err: reqwest::Error) -> Self {
        Self::new(BoticelliErrorKind::from(err))
    }
}

impl From<serde_json::Error> for BoticelliError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(BoticelliErrorKind::from(err))
    }
}

#[cfg(feature = "gemini")]
impl From<crate::models::gemini::GeminiError> for BoticelliError {
    fn from(err: crate::models::gemini::GeminiError) -> Self {
        Self::new(BoticelliErrorKind::from(err))
    }
}

impl From<crate::narrative::NarrativeError> for BoticelliError {
    fn from(err: crate::narrative::NarrativeError) -> Self {
        Self::new(BoticelliErrorKind::from(err))
    }
}

#[cfg(feature = "database")]
impl From<diesel::result::Error> for BoticelliError {
    fn from(err: diesel::result::Error) -> Self {
        Self::new(BoticelliErrorKind::Backend(format!(
            "Database error: {}",
            err
        )))
    }
}

/// Result type for Boticelli operations.
pub type BoticelliResult<T> = std::result::Result<T, BoticelliError>;
