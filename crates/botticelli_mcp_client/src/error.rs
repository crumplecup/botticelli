//! Error types for MCP client operations.

use derive_more::{Display, Error};

/// Specific error conditions for the Botticelli MCP client.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum McpClientErrorKind {
    /// Connection to server failed.
    #[display("Connection error: {}", _0)]
    ConnectionError(String),

    /// Tool execution failed.
    #[display("Tool execution failed: {}", _0)]
    ToolExecutionFailed(String),

    /// Serialization or deserialization error.
    #[display("Serialization error: {}", _0)]
    SerializationError(String),

    /// Configuration error.
    #[display("Configuration error: {}", _0)]
    Configuration(String),
}

/// MCP client error with location tracking.
#[derive(Debug, Clone, Display, Error)]
#[display("MCP Client Error: {} at {}:{}", kind, file, line)]
pub struct McpClientError {
    /// The specific error kind.
    pub kind: McpClientErrorKind,
    /// Line number where error occurred.
    pub line: u32,
    /// File where error occurred.
    pub file: &'static str,
}

impl McpClientError {
    /// Creates a new error with automatic location tracking.
    #[track_caller]
    pub fn new(kind: McpClientErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

/// Result type for MCP client operations.
pub type McpClientResult<T> = Result<T, McpClientError>;

impl From<McpClientErrorKind> for McpClientError {
    #[track_caller]
    fn from(kind: McpClientErrorKind) -> Self {
        McpClientError::new(kind)
    }
}
