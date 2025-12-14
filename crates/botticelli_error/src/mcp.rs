//! MCP-specific error types.

/// MCP error kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum McpErrorKind {
    /// Tool not found
    #[display("Tool not found: {}", _0)]
    ToolNotFound(String),
    /// Invalid tool arguments
    #[display("Invalid arguments for tool {}: {}", tool, reason)]
    InvalidArguments {
        /// Tool name
        tool: String,
        /// Reason for invalidity
        reason: String,
    },
    /// Invalid input
    #[display("Invalid input: {}", _0)]
    InvalidInput(String),
    /// Tool execution failed
    #[display("Tool execution failed: {}", _0)]
    ExecutionFailed(String),
    /// Tool execution failed (alias)
    #[display("Tool execution failed: {}", _0)]
    ToolExecutionFailed(String),
    /// Execution error (alias)
    #[display("Execution error: {}", _0)]
    ExecutionError(String),
    /// Invalid state for operation
    #[display("Invalid state: {}", _0)]
    InvalidState(String),
    /// Session error
    #[display("Session error: {}", _0)]
    Session(String),
    /// Resource not found
    #[display("Resource not found: {}", _0)]
    ResourceNotFound(String),
    /// Backend unavailable
    #[display("Backend unavailable: {}", _0)]
    BackendUnavailable(String),
    /// Unsupported model
    #[display("Unsupported model: {}", _0)]
    UnsupportedModel(String),
    /// Serialization error
    #[display("Serialization error: {}", _0)]
    SerializationError(String),
    /// Mutex poisoned (internal error)
    #[display("Mutex poisoned: {}", _0)]
    MutexPoisoned(String),
}

/// MCP error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("MCP Error: {} at {}:{}", kind, file, line)]
pub struct McpError {
    /// Error kind
    pub kind: McpErrorKind,
    /// Line number
    pub line: u32,
    /// File path
    pub file: &'static str,
}

impl McpError {
    /// Create a new MCP error.
    #[track_caller]
    pub fn new(kind: McpErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }

    /// Create a tool not found error.
    #[track_caller]
    pub fn tool_not_found(tool: impl Into<String>) -> Self {
        Self::new(McpErrorKind::ToolNotFound(tool.into()))
    }

    /// Create an invalid arguments error.
    #[track_caller]
    pub fn invalid_arguments(tool: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(McpErrorKind::InvalidArguments {
            tool: tool.into(),
            reason: reason.into(),
        })
    }

    /// Create an invalid input error.
    #[track_caller]
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::InvalidInput(message.into()))
    }

    /// Create an execution failed error.
    #[track_caller]
    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::ExecutionFailed(message.into()))
    }

    /// Create an invalid state error.
    #[track_caller]
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::InvalidState(message.into()))
    }

    /// Create a session error.
    #[track_caller]
    pub fn session(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Session(message.into()))
    }

    /// Create a session not found error.
    #[track_caller]
    pub fn session_not_found(session_id: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Session(format!(
            "Session not found: {}",
            session_id.into()
        )))
    }

    /// Create a resource not found error.
    #[track_caller]
    pub fn resource_not_found(resource: impl Into<String>) -> Self {
        Self::new(McpErrorKind::ResourceNotFound(resource.into()))
    }

    /// Create a backend unavailable error.
    #[track_caller]
    pub fn backend_unavailable(backend: impl Into<String>) -> Self {
        Self::new(McpErrorKind::BackendUnavailable(backend.into()))
    }

    /// Create an unsupported model error.
    #[track_caller]
    pub fn unsupported_model(model: impl Into<String>) -> Self {
        Self::new(McpErrorKind::UnsupportedModel(model.into()))
    }

    /// Create a mutex poisoned error.
    #[track_caller]
    pub fn mutex_poisoned(context: impl Into<String>) -> Self {
        Self::new(McpErrorKind::MutexPoisoned(context.into()))
    }
}

/// Result type for MCP operations.
pub type McpResult<T> = std::result::Result<T, McpError>;
