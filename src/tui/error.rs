//! TUI error types.

/// TUI error kind variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiErrorKind {
    /// Failed to set up terminal (enable raw mode, alternate screen, etc.)
    TerminalSetup(String),
    /// Failed to restore terminal to original state
    TerminalRestore(String),
    /// Failed to poll for terminal events
    EventPoll(String),
    /// Failed to read terminal event
    EventRead(String),
    /// Failed to render TUI frame
    Rendering(String),
}

impl std::fmt::Display for TuiErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TuiErrorKind::TerminalSetup(msg) => write!(f, "Failed to set up terminal: {}", msg),
            TuiErrorKind::TerminalRestore(msg) => {
                write!(f, "Failed to restore terminal: {}", msg)
            }
            TuiErrorKind::EventPoll(msg) => write!(f, "Failed to poll for events: {}", msg),
            TuiErrorKind::EventRead(msg) => write!(f, "Failed to read event: {}", msg),
            TuiErrorKind::Rendering(msg) => write!(f, "Failed to render: {}", msg),
        }
    }
}

/// TUI error with source location tracking.
#[derive(Debug, Clone)]
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

impl std::fmt::Display for TuiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TUI Error: {} at line {} in {}",
            self.kind, self.line, self.file
        )
    }
}

impl std::error::Error for TuiError {}
