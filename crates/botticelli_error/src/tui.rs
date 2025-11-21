//! TUI (Terminal User Interface) error types.

/// TUI error kind variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub enum TuiErrorKind {
    /// Failed to set up terminal (enable raw mode, alternate screen, etc.)
    #[display("Failed to set up terminal: {}", _0)]
    TerminalSetup(String),
    /// Failed to restore terminal to original state
    #[display("Failed to restore terminal: {}", _0)]
    TerminalRestore(String),
    /// Failed to poll for terminal events
    #[display("Failed to poll for events: {}", _0)]
    EventPoll(String),
    /// Failed to read terminal event
    #[display("Failed to read event: {}", _0)]
    EventRead(String),
    /// Failed to render TUI frame
    #[display("Failed to render: {}", _0)]
    Rendering(String),
    /// Database operation failed
    #[display("Database error: {}", _0)]
    Database(String),
}

/// TUI error with source location tracking.
///
/// # Examples
///
/// ```
/// use botticelli_error::{TuiError, TuiErrorKind};
///
/// let err = TuiError::new(TuiErrorKind::TerminalSetup("Raw mode failed".to_string()));
/// assert!(format!("{}", err).contains("terminal"));
/// ```
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
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

/// Result type for TUI operations.
pub type TuiResult<T> = Result<T, TuiError>;
