//! Error types for MCP server.

/// Errors that can occur in the MCP server.
#[derive(Debug, Clone, derive_more::Display)]
pub enum McpError {
    /// Tool not found
    #[display("Tool not found: {}", _0)]
    ToolNotFound(String),

    /// Tool execution failed
    #[display("Tool execution failed: {}", _0)]
    ToolExecutionFailed(String),

    /// Invalid tool input
    #[display("Invalid tool input: {}", _0)]
    InvalidInput(String),

    /// Resource not found
    #[display("Resource not found: {}", _0)]
    ResourceNotFound(String),

    /// Server initialization failed
    #[display("Server initialization failed: {}", _0)]
    InitializationFailed(String),

    /// Transport error
    #[display("Transport error: {}", _0)]
    TransportError(String),

    /// Tool not allowed
    #[display("Tool not allowed: {}", _0)]
    ToolNotAllowed(String),

    /// Backend unavailable
    #[display("Backend unavailable: {}", _0)]
    BackendUnavailable(String),

    /// Unsupported model
    #[display("Unsupported model: {}", _0)]
    UnsupportedModel(String),

    /// Execution error
    #[display("Execution error: {}", _0)]
    ExecutionError(String),
}

impl std::error::Error for McpError {}

/// Result type for MCP operations.
pub type McpResult<T> = Result<T, McpError>;
