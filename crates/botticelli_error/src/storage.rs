//! Storage error types.

/// Kinds of storage errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub enum StorageErrorKind {
    /// Failed to create storage directory
    #[display("Failed to create storage directory: {}", _0)]
    DirectoryCreation(String),
    /// Failed to write file
    #[display("Failed to write file: {}", _0)]
    FileWrite(String),
    /// Failed to read file
    #[display("Failed to read file: {}", _0)]
    FileRead(String),
    /// Media not found at the specified location
    #[display("Media not found: {}", _0)]
    NotFound(String),
    /// Invalid storage path
    #[display("Invalid storage path: {}", _0)]
    InvalidPath(String),
    /// Permission denied when accessing storage
    #[display("Permission denied: {}", _0)]
    PermissionDenied(String),
    /// Invalid storage configuration
    #[display("Invalid configuration: {}", _0)]
    InvalidConfig(String),
    /// Storage backend is unavailable
    #[display("Storage unavailable: {}", _0)]
    Unavailable(String),
}

/// Storage error with location tracking.
///
/// # Examples
///
/// ```
/// use botticelli_error::{StorageError, StorageErrorKind};
///
/// let err = StorageError::new(StorageErrorKind::NotFound("/path/to/file".to_string()));
/// assert!(format!("{}", err).contains("not found"));
/// ```
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Storage Error: {} at line {} in {}", kind, line, file)]
pub struct StorageError {
    /// The kind of error that occurred
    pub kind: StorageErrorKind,
    /// Line number where error was created
    pub line: u32,
    /// File where error was created
    pub file: &'static str,
}

impl StorageError {
    /// Create a new storage error with automatic location tracking.
    #[track_caller]
    pub fn new(kind: StorageErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}
