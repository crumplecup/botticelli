//! TUI (Terminal User Interface) error types.

/// IO error with source tracking for TUI operations.
#[cfg(feature = "mcp")]
use crate::tool;

use elicitation::Prompt;

#[derive(
    Debug, derive_more::Display, derive_more::Error, derive_getters::Getters, elicitation::Elicit,
)]
#[display("IO error: {:?} at {}:{}", source, file, line)]
pub struct TuiIoError {
    /// The io::Error source
    source: Box<std::io::Error>,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

impl TuiIoError {
    /// Create a new TuiIoError with automatic location tracking.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(err: std::io::Error) -> Self {
        let location = std::panic::Location::caller();
        Self {
            source: Box::new(err),
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

impl Clone for TuiIoError {
    fn clone(&self) -> Self {
        // std::io::Error is not Clone, reconstruct with same kind and message
        let kind = self.source.kind();
        let msg = format!("{:?}", self.source);
        Self {
            source: Box::new(std::io::Error::new(kind, msg)),
            line: self.line,
            file: self.file.clone(),
        }
    }
}

impl serde::Serialize for TuiIoError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("TuiIoError", 3)?;
        state.serialize_field("source", &format!("{:?}", self.source))?;
        state.serialize_field("line", &self.line)?;
        state.serialize_field("file", &self.file)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for TuiIoError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            source: String,
            line: u32,
            file: String,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(Self {
            source: Box::new(std::io::Error::other(helper.source)),
            line: helper.line,
            file: helper.file,
        })
    }
}

impl schemars::JsonSchema for TuiIoError {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "TuiIoError".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "object",
            "required": ["source", "line", "file"],
            "properties": {
                "source": { "type": "string" },
                "line": generator.subschema_for::<u32>(),
                "file": generator.subschema_for::<String>()
            }
        })
    }
}

/// TUI error kind variants.
#[derive(Debug, Clone, derive_more::Display)]
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
    /// IO error
    #[display("{}", _0)]
    Io(TuiIoError),
    /// Database operation failed
    #[display("Database error: {}", _0)]
    Database(String),
    /// Conversation storage operation failed
    #[display("Storage error: {}", _0)]
    Storage(String),
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
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("TUI Error: {} at line {} in {}", kind, line, file)]
pub struct TuiError {
    /// Error kind
    kind: TuiErrorKind,
    /// Line number where error occurred
    line: u32,
    /// File where error occurred
    file: String,
}

impl TuiError {
    /// Create a new TuiError with automatic location tracking.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(kind: TuiErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

crate::impl_error_from_kind!(TuiErrorKind => TuiError);

impl From<std::io::Error> for TuiError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        Self::new(TuiErrorKind::Io(TuiIoError::new(err)))
    }
}

// Bridge std::io::Error to BotticelliErrorKind
#[cfg(feature = "tui")]
crate::bridge_error!(std::io::Error => TuiError => crate::BotticelliErrorKind);

/// Result type for TUI operations.
pub type TuiResult<T> = Result<T, TuiError>;
