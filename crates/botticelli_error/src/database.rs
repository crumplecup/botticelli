//! Database error types.

#[cfg(feature = "mcp")]
use crate::tool;

use std::sync::Arc;

#[cfg(feature = "serde_json")]
use crate::json::SerdeJsonError;

/// Diesel-specific error with source tracking.
#[cfg(feature = "database")]
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Diesel error: {:?} at {}:{}", source, file, line)]
pub struct DieselError {
    /// The diesel error source
    source: Arc<diesel::result::Error>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

#[cfg(feature = "database")]
impl DieselError {
    /// Create a new DieselError with automatic location tracking.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(err: diesel::result::Error) -> Self {
        let location = std::panic::Location::caller();
        Self {
            source: Arc::new(err),
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

#[cfg(feature = "database")]
impl Clone for DieselError {
    fn clone(&self) -> Self {
        Self {
            source: Arc::clone(&self.source),
            line: self.line,
            file: self.file.clone(),
        }
    }
}

#[cfg(feature = "database")]
impl PartialEq for DieselError {
    fn eq(&self, other: &Self) -> bool {
        // Compare by string representation since diesel::Error doesn't impl PartialEq
        format!("{:?}", self.source) == format!("{:?}", other.source)
            && self.line == other.line
            && self.file == other.file
    }
}

#[cfg(feature = "database")]
impl Eq for DieselError {}

#[cfg(feature = "database")]
impl std::hash::Hash for DieselError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{:?}", self.source).hash(state);
        self.line.hash(state);
        self.file.hash(state);
    }
}

/// Diesel connection error with source tracking.
#[cfg(feature = "database")]
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Diesel connection error: {:?} at {}:{}", source, file, line)]
pub struct DieselConnectionError {
    /// The diesel connection error source
    source: Arc<diesel::ConnectionError>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

#[cfg(feature = "database")]
impl DieselConnectionError {
    /// Create a new DieselConnectionError with automatic location tracking.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(err: diesel::ConnectionError) -> Self {
        let location = std::panic::Location::caller();
        Self {
            source: Arc::new(err),
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

#[cfg(feature = "database")]
impl Clone for DieselConnectionError {
    fn clone(&self) -> Self {
        Self {
            source: Arc::clone(&self.source),
            line: self.line,
            file: self.file.clone(),
        }
    }
}

#[cfg(feature = "database")]
impl PartialEq for DieselConnectionError {
    fn eq(&self, other: &Self) -> bool {
        // Compare by string representation since diesel::ConnectionError doesn't impl PartialEq
        format!("{:?}", self.source) == format!("{:?}", other.source)
            && self.line == other.line
            && self.file == other.file
    }
}

#[cfg(feature = "database")]
impl Eq for DieselConnectionError {}

#[cfg(feature = "database")]
impl std::hash::Hash for DieselConnectionError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{:?}", self.source).hash(state);
        self.line.hash(state);
        self.file.hash(state);
    }
}

/// r2d2 pool error with source tracking.
#[cfg(feature = "database")]
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("R2D2 pool error: {:?} at {}:{}", source, file, line)]
pub struct R2d2Error {
    /// The r2d2 error source
    source: Arc<r2d2::Error>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

#[cfg(feature = "database")]
impl R2d2Error {
    /// Create a new R2d2Error with automatic location tracking.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(err: r2d2::Error) -> Self {
        let location = std::panic::Location::caller();
        Self {
            source: Arc::new(err),
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

#[cfg(feature = "database")]
impl Clone for R2d2Error {
    fn clone(&self) -> Self {
        Self {
            source: Arc::clone(&self.source),
            line: self.line,
            file: self.file.clone(),
        }
    }
}

#[cfg(feature = "database")]
impl PartialEq for R2d2Error {
    fn eq(&self, other: &Self) -> bool {
        // Compare by string representation since r2d2::Error doesn't impl PartialEq
        format!("{:?}", self.source) == format!("{:?}", other.source)
            && self.line == other.line
            && self.file == other.file
    }
}

#[cfg(feature = "database")]
impl Eq for R2d2Error {}

#[cfg(feature = "database")]
impl std::hash::Hash for R2d2Error {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{:?}", self.source).hash(state);
        self.line.hash(state);
        self.file.hash(state);
    }
}

/// Database error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum DatabaseErrorKind {
    /// Connection failed
    #[display("Database connection error: {}", _0)]
    Connection(String),

    /// Diesel connection error
    #[cfg(feature = "database")]
    #[display("{}", _0)]
    DieselConnection(DieselConnectionError),

    /// R2D2 pool error
    #[cfg(feature = "database")]
    #[display("{}", _0)]
    R2d2(R2d2Error),

    /// Query execution failed
    #[display("Database query error: {}", _0)]
    Query(String),

    /// Diesel query error
    #[cfg(feature = "database")]
    #[display("{}", _0)]
    Diesel(DieselError),

    /// Serialization/deserialization error
    #[display("Serialization error: {}", _0)]
    Serialization(String),

    /// Serde JSON error
    #[cfg(feature = "serde_json")]
    #[display("{}", _0)]
    SerdeJson(SerdeJsonError),

    /// Migration error
    #[display("Migration error: {}", _0)]
    Migration(String),

    /// Record not found
    #[display("Record not found")]
    NotFound,

    /// Table not found
    #[display("Table '{}' not found in database", _0)]
    TableNotFound(String),

    /// Schema inference error
    #[display("Schema inference error: {}", _0)]
    SchemaInference(String),

    /// Invalid query
    #[display("Invalid query: {}", _0)]
    InvalidQuery(String),
}

/// Database error with source location tracking.
///
/// # Examples
///
/// ```
/// use botticelli_error::{DatabaseError, DatabaseErrorKind};
///
/// let err = DatabaseError::new(DatabaseErrorKind::NotFound);
/// assert!(format!("{}", err).contains("not found"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::Error)]
#[display("Database Error: {} at line {} in {}", kind, line, file)]
pub struct DatabaseError {
    /// The kind of error that occurred
    pub kind: DatabaseErrorKind,
    /// Line number where error was created
    pub line: u32,
    /// File where error was created
    pub file: String,
}

impl DatabaseError {
    /// Create a new DatabaseError with automatic location tracking.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(kind: DatabaseErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

crate::impl_error_from_kind!(DatabaseErrorKind => DatabaseError);

// Diesel error conversions (only available with database feature)
#[cfg(feature = "database")]
impl From<diesel::result::Error> for DatabaseError {
    #[track_caller]
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => DatabaseError::new(DatabaseErrorKind::NotFound),
            _ => DatabaseError::new(DatabaseErrorKind::Diesel(DieselError::new(err))),
        }
    }
}

#[cfg(feature = "database")]
impl From<diesel::ConnectionError> for DatabaseError {
    #[track_caller]
    fn from(err: diesel::ConnectionError) -> Self {
        DatabaseError::new(DatabaseErrorKind::DieselConnection(
            DieselConnectionError::new(err),
        ))
    }
}

#[cfg(feature = "database")]
impl From<r2d2::Error> for DatabaseError {
    #[track_caller]
    fn from(err: r2d2::Error) -> Self {
        DatabaseError::new(DatabaseErrorKind::R2d2(R2d2Error::new(err)))
    }
}

#[cfg(feature = "serde_json")]
impl From<serde_json::Error> for DatabaseError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        DatabaseError::new(DatabaseErrorKind::SerdeJson(SerdeJsonError::new(err)))
    }
}

// Bridge external errors to BotticelliErrorKind
#[cfg(feature = "database")]
crate::bridge_error!(diesel::result::Error => DatabaseError => crate::BotticelliErrorKind);

#[cfg(feature = "database")]
crate::bridge_error!(diesel::ConnectionError => DatabaseError => crate::BotticelliErrorKind);
