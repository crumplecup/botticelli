//! TUI (Terminal User Interface) error types.

use crate::{BotticelliError, BuilderError, GeminiError};

/// TUI error kind variants.
///
/// Every variant holds the **original typed error** (not a stringified representation)
/// so the full error chain is preserved for logging and diagnostics.
#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum TuiErrorKind {
    /// Terminal or file I/O failure.
    #[display("I/O error: {}", _0)]
    Io(std::io::Error),

    /// Upstream Botticelli error (driver load, model initialisation, etc.)
    #[display("Botticelli error: {}", _0)]
    Botticelli(BotticelliError),

    /// Gemini client error.
    #[display("Gemini error: {}", _0)]
    Gemini(GeminiError),

    /// Builder or configuration validation error.
    #[display("Builder error: {}", _0)]
    Builder(BuilderError),

    /// Missing or invalid environment variable (e.g. API key).
    #[display("Environment variable error: {}", _0)]
    EnvVar(std::env::VarError),

    /// A code path that is statically unreachable was reached at runtime.
    ///
    /// Used instead of `unreachable!()` so the invariant violation is surfaced
    /// as a structured error rather than a panic.
    #[display("Unreachable invariant violated: {message}")]
    Unreachable {
        /// Description of the violated invariant.
        message: String,
    },
}

/// TUI error with source location tracking.
///
/// # Examples
///
/// ```
/// use botticelli_error::{TuiError, TuiErrorKind};
/// use std::io;
///
/// let err = TuiError::from(io::Error::new(io::ErrorKind::PermissionDenied, "raw mode failed"));
/// assert!(format!("{}", err).contains("I/O"));
/// ```
#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("TUI Error: {} at line {} in {}", kind, line, file)]
pub struct TuiError {
    /// Error kind
    pub kind: TuiErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// File where error occurred
    pub file: &'static str,
}

impl TuiError {
    /// Create a new TuiError with automatic location tracking.
    #[track_caller]
    pub fn new(kind: TuiErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

impl From<std::io::Error> for TuiError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind: TuiErrorKind::Io(err),
            line: loc.line(),
            file: loc.file(),
        }
    }
}

impl From<BotticelliError> for TuiError {
    #[track_caller]
    fn from(err: BotticelliError) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind: TuiErrorKind::Botticelli(err),
            line: loc.line(),
            file: loc.file(),
        }
    }
}

impl From<GeminiError> for TuiError {
    #[track_caller]
    fn from(err: GeminiError) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind: TuiErrorKind::Gemini(err),
            line: loc.line(),
            file: loc.file(),
        }
    }
}

impl From<BuilderError> for TuiError {
    #[track_caller]
    fn from(err: BuilderError) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind: TuiErrorKind::Builder(err),
            line: loc.line(),
            file: loc.file(),
        }
    }
}

impl From<std::env::VarError> for TuiError {
    #[track_caller]
    fn from(err: std::env::VarError) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind: TuiErrorKind::EnvVar(err),
            line: loc.line(),
            file: loc.file(),
        }
    }
}

/// Result type for TUI operations.
pub type TuiResult<T> = Result<T, TuiError>;
