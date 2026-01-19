//! Environment variable errors.

use std::env::VarError;

/// Environment variable error kind.
#[derive(Debug, Clone, derive_more::Display)]
pub enum EnvErrorKind {
    /// Environment variable not found.
    #[display("Environment variable '{}' not found", _0)]
    NotFound(String),

    /// Environment variable contains invalid unicode.
    #[display("Environment variable '{}' contains invalid unicode", _0)]
    InvalidUnicode(String),
}

/// Environment variable error.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Environment error: {} at {}:{}", kind, file, line)]
pub struct EnvError {
    kind: EnvErrorKind,
    line: u32,
    file: String,
}

impl EnvError {
    /// Creates a new environment error.
    #[track_caller]
    pub fn new(kind: EnvErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }

    /// Gets the error kind.
    pub fn kind(&self) -> &EnvErrorKind {
        &self.kind
    }
}

impl From<EnvErrorKind> for EnvError {
    #[track_caller]
    fn from(kind: EnvErrorKind) -> Self {
        Self::new(kind)
    }
}

impl From<VarError> for EnvError {
    #[track_caller]
    fn from(err: VarError) -> Self {
        let kind = match err {
            VarError::NotPresent => EnvErrorKind::NotFound("unknown".to_string()),
            VarError::NotUnicode(_) => EnvErrorKind::InvalidUnicode("unknown".to_string()),
        };
        Self::new(kind)
    }
}
