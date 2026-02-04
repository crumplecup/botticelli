//! Chat-specific error types.

#[cfg(feature = "mcp")]
use crate::tool;

use elicitation::{Prompt, Select};

/// Sampling error kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, elicitation::Elicit)]
pub enum SamplingErrorKind {
    /// Max turns exceeded
    #[display("Max turns exceeded: {}", max)]
    MaxTurnsExceeded {
        /// Maximum turns allowed
        max: usize,
    },

    /// Tool execution failed
    #[display("Tool execution failed: {} - {}", tool_name, reason)]
    ToolExecutionFailed {
        /// Tool name
        tool_name: String,
        /// Reason for failure
        reason: String,
    },

    /// Unknown tool
    #[display("Unknown tool: {}", name)]
    UnknownTool {
        /// Tool name
        name: String,
    },

    /// Provider error
    #[display("Provider error: {}", _0)]
    ProviderError(String),

    /// No tool registry configured
    #[display("No tool registry configured")]
    NoToolRegistry,

    /// Request building failed
    #[display("Request building failed: {}", _0)]
    RequestBuildingFailed(String),
}

/// Sampling error with location tracking.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
    derive_more::Error,
    derive_getters::Getters,
    elicitation::Elicit,
)]
#[display("Sampling: {} at {}:{}", kind, file, line)]
pub struct SamplingError {
    /// Error kind
    kind: SamplingErrorKind,
    /// Line number
    line: u32,
    /// File name
    file: String,
}

impl SamplingError {
    /// Create a new sampling error with location tracking.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(kind: SamplingErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }
}

/// Chat error kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, elicitation::Elicit)]
pub enum ChatErrorKind {
    /// Command not found
    #[display("Command not found: {}", _0)]
    CommandNotFound(String),
    /// Invalid command arguments
    #[display("Invalid arguments for command {}: {}", command, reason)]
    InvalidArguments {
        /// Command name
        command: String,
        /// Reason for invalidity
        reason: String,
    },
    /// Missing required argument
    #[display("Missing required argument: {}", _0)]
    MissingArgument(String),
    /// Parse error
    #[display("Parse error: {}", _0)]
    ParseError(String),
    /// Invalid input
    #[display("Invalid input: {}", _0)]
    InvalidInput(String),
    /// Invalid state for operation
    #[display("Invalid state: {}", _0)]
    InvalidState(String),
    /// Command execution failed
    #[display("Command execution failed: {}", _0)]
    ExecutionFailed(String),
    /// Dialog interaction failed
    #[display("Dialog interaction failed: {}", _0)]
    DialogFailed(String),
    /// Validation error
    #[display("Validation error: {}", _0)]
    ValidationError(String),
    /// User cancelled operation
    #[display("User cancelled operation")]
    UserCancelled,
    /// Feature not implemented
    #[display("Not implemented: {}", _0)]
    NotImplemented(String),
    /// I/O error
    #[display("I/O error: {}", _0)]
    IoError(String),
    /// Sampling error
    #[display("Sampling error: {}", _0)]
    Sampling(SamplingError),
}

/// From implementation for automatic conversion
impl From<SamplingError> for ChatErrorKind {
    fn from(err: SamplingError) -> Self {
        Self::Sampling(err)
    }
}

impl From<SamplingErrorKind> for ChatErrorKind {
    fn from(kind: SamplingErrorKind) -> Self {
        Self::Sampling(SamplingError::new(kind))
    }
}

/// Chat error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Chat Error: {} at {}:{}", kind, file, line)]
pub struct ChatError {
    /// Error kind
    pub kind: ChatErrorKind,
    /// Line number
    pub line: u32,
    /// File path
    pub file: String,
}

impl ChatError {
    /// Create a new chat error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(kind: ChatErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }

    /// Create a command not found error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn command_not_found(command: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::CommandNotFound(command.into()))
    }

    /// Create an invalid arguments error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn invalid_arguments(command: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::InvalidArguments {
            command: command.into(),
            reason: reason.into(),
        })
    }

    /// Create an execution failed error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::ExecutionFailed(message.into()))
    }

    /// Create a dialog failed error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn dialog_failed(message: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::DialogFailed(message.into()))
    }

    /// Create a user cancelled error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn user_cancelled() -> Self {
        Self::new(ChatErrorKind::UserCancelled)
    }

    /// Create a missing argument error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn missing_argument(arg: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::MissingArgument(arg.into()))
    }

    /// Create a parse error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::ParseError(message.into()))
    }

    /// Create an invalid input error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::InvalidInput(message.into()))
    }

    /// Create an invalid state error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::InvalidState(message.into()))
    }

    /// Create a validation error.
#[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new(ChatErrorKind::ValidationError(message.into()))
    }
}

/// Result type for chat operations.
pub type ChatResult<T> = std::result::Result<T, ChatError>;
