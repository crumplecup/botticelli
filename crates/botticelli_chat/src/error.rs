//! Error types for botticelli_chat.

/// Specific error conditions in chat operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ChatErrorKind {
    /// Invalid input provided.
    #[display("Invalid input: {}", _0)]
    InvalidInput(String),

    /// Command parsing failed.
    #[display("Failed to parse command: {}", _0)]
    ParseError(String),

    /// Unknown command.
    #[display("Unknown command: {}", _0)]
    UnknownCommand(String),

    /// Missing required argument.
    #[display("Missing required argument: {}", _0)]
    MissingArgument(String),

    /// Invalid state transition.
    #[display("Invalid state transition: {}", _0)]
    InvalidState(String),

    /// IO error occurred.
    #[display("IO error: {}", _0)]
    IoError(String),

    /// Serialization error.
    #[display("Serialization error: {}", _0)]
    SerializationError(String),
}

/// Chat error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Chat error: {} at {}:{}", kind, file, line)]
pub struct ChatError {
    /// The specific error condition.
    pub kind: ChatErrorKind,
    /// Line number where error occurred.
    pub line: u32,
    /// Source file where error occurred.
    pub file: &'static str,
}

impl ChatError {
    /// Create a new ChatError with location tracking.
    #[track_caller]
    pub fn new(kind: ChatErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }

    /// Create an invalid input error.
    #[track_caller]
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::InvalidInput(msg.into()))
    }

    /// Create a parse error.
    #[track_caller]
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::ParseError(msg.into()))
    }

    /// Create an unknown command error.
    #[track_caller]
    pub fn unknown_command(cmd: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::UnknownCommand(cmd.into()))
    }

    /// Create a missing argument error.
    #[track_caller]
    pub fn missing_argument(arg: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::MissingArgument(arg.into()))
    }

    /// Create an invalid state error.
    #[track_caller]
    pub fn invalid_state(msg: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::InvalidState(msg.into()))
    }

    /// Create an IO error.
    #[track_caller]
    pub fn io_error(msg: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::IoError(msg.into()))
    }

    /// Create a serialization error.
    #[track_caller]
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::SerializationError(msg.into()))
    }
}

impl From<std::io::Error> for ChatError {
    fn from(err: std::io::Error) -> Self {
        Self::io_error(err.to_string())
    }
}

impl From<toml::de::Error> for ChatError {
    fn from(err: toml::de::Error) -> Self {
        Self::serialization_error(err.to_string())
    }
}

impl From<toml::ser::Error> for ChatError {
    fn from(err: toml::ser::Error) -> Self {
        Self::serialization_error(err.to_string())
    }
}

/// Result type for chat operations.
pub type ChatResult<T> = Result<T, ChatError>;
