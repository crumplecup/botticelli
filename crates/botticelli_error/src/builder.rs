//! Builder-related errors.

/// Specific builder error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum BuilderErrorKind {
    /// Missing required field
    #[display("Missing required field: {}", _0)]
    MissingField(String),
    
    /// Invalid field value
    #[display("Invalid field value for '{}': {}", field, reason)]
    InvalidField {
        /// The field name
        field: String,
        /// Reason for invalidity
        reason: String
    },
    
    /// Validation failed
    #[display("Validation failed: {}", _0)]
    ValidationFailed(String),
}

/// Builder error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Builder Error: {} at line {} in {}", kind, line, file)]
pub struct BuilderError {
    kind: BuilderErrorKind,
    line: u32,
    file: &'static str,
}

impl BuilderError {
    /// Create a new builder error with caller location tracking.
    #[track_caller]
    pub fn new(kind: BuilderErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
    
    /// Get the error kind.
    pub fn kind(&self) -> &BuilderErrorKind {
        &self.kind
    }
}

/// Convert from derive_builder error string.
impl From<String> for BuilderError {
    #[track_caller]
    fn from(msg: String) -> Self {
        Self::new(BuilderErrorKind::ValidationFailed(msg))
    }
}

/// Convert from derive_builder error &str.
impl From<&str> for BuilderError {
    #[track_caller]
    fn from(msg: &str) -> Self {
        Self::new(BuilderErrorKind::ValidationFailed(msg.to_string()))
    }
}
