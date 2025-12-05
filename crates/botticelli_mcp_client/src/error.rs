//! Error types for MCP client operations.

use derive_more::{Display, Error};

/// Specific error conditions for MCP client operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum McpClientErrorKind {
    /// Tool execution failed.
    #[display("Tool execution failed: {}", _0)]
    ToolExecutionFailed(String),

    /// Invalid tool call from LLM.
    #[display("Invalid tool call: {}", _0)]
    InvalidToolCall(String),

    /// Tool not found.
    #[display("Tool not found: {}", _0)]
    ToolNotFound(String),

    /// LLM backend error.
    #[display("LLM error: {}", _0)]
    LlmError(String),

    /// Serialization error.
    #[display("Serialization error: {}", _0)]
    SerializationError(String),

    /// Maximum iterations exceeded.
    #[display("Maximum iterations exceeded: {}", _0)]
    MaxIterationsExceeded(usize),

    /// Connection error.
    #[display("Connection error: {}", _0)]
    ConnectionError(String),

    /// Timeout error.
    #[display("Timeout: {}", _0)]
    Timeout(String),

    /// Rate limit exceeded.
    #[display("Rate limit exceeded: {}", _0)]
    RateLimitExceeded(String),

    /// Circuit breaker open.
    #[display("Circuit breaker open for: {}", _0)]
    CircuitBreakerOpen(String),

    /// Metrics registration error.
    #[display("Metrics error: {}", _0)]
    MetricsError(String),
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

impl From<prometheus::Error> for McpClientError {
    fn from(err: prometheus::Error) -> Self {
        McpClientError::new(McpClientErrorKind::MetricsError(err.to_string()))
    }
}

impl McpClientErrorKind {
    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ConnectionError(_) | Self::Timeout(_) | Self::ToolExecutionFailed(_)
        )
    }

    /// Returns true if this error should trigger backoff.
    pub fn should_backoff(&self) -> bool {
        matches!(self, Self::RateLimitExceeded(_))
    }
}
