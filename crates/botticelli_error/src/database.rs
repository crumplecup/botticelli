//! Database error types.

/// Database error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub enum DatabaseErrorKind {
    /// Connection failed
    #[display("Database connection error: {}", _0)]
    Connection(String),
    /// Query execution failed
    #[display("Database query error: {}", _0)]
    Query(String),
    /// Serialization/deserialization error
    #[display("Serialization error: {}", _0)]
    Serialization(String),
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
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Database Error: {} at line {} in {}", kind, line, file)]
pub struct DatabaseError {
    /// The kind of error that occurred
    pub kind: DatabaseErrorKind,
    /// Line number where error was created
    pub line: u32,
    /// File where error was created
    pub file: &'static str,
}

impl DatabaseError {
    /// Create a new DatabaseError with automatic location tracking.
    #[track_caller]
    pub fn new(kind: DatabaseErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

// Diesel error conversions (only available with database feature)
#[cfg(feature = "database")]
impl From<diesel::result::Error> for DatabaseError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => DatabaseError::new(DatabaseErrorKind::NotFound),
            _ => DatabaseError::new(DatabaseErrorKind::Query(err.to_string())),
        }
    }
}

#[cfg(feature = "database")]
impl From<diesel::ConnectionError> for DatabaseError {
    fn from(err: diesel::ConnectionError) -> Self {
        DatabaseError::new(DatabaseErrorKind::Connection(err.to_string()))
    }
}

#[cfg(feature = "database")]
impl From<serde_json::Error> for DatabaseError {
    fn from(err: serde_json::Error) -> Self {
        DatabaseError::new(DatabaseErrorKind::Serialization(err.to_string()))
    }
}
