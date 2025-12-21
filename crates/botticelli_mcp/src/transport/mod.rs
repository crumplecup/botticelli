//! Transport abstraction for MCP clients.

mod http;

pub use http::HttpTransport;

use async_trait::async_trait;
use botticelli_core::ToolDefinition;
use serde_json::Value;

/// Transport-agnostic MCP client interface.
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Initialize connection to MCP server.
    async fn initialize(&mut self) -> Result<(), McpTransportError>;

    /// List available tools from the server.
    async fn list_tools(&self) -> Result<Vec<ToolDefinition>, McpTransportError>;

    /// Call a tool with the given arguments.
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, McpTransportError>;

    /// Check if the transport is connected.
    fn is_connected(&self) -> bool;
}

/// Transport layer error kinds.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum McpTransportErrorKind {
    /// Connection failed
    #[display("Connection failed: {}", _0)]
    ConnectionFailed(String),

    /// Request failed
    #[display("Request failed: {}", _0)]
    RequestFailed(String),

    /// Invalid response
    #[display("Invalid response: {}", _0)]
    InvalidResponse(String),

    /// Not initialized
    #[display("Transport not initialized")]
    NotInitialized,
}

/// Transport layer errors with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Transport: {} at {}:{}", kind, file, line)]
pub struct McpTransportError {
    /// Error kind
    pub kind: McpTransportErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// File where error occurred
    pub file: &'static str,
}

impl McpTransportError {
    /// Create a new transport error.
    #[track_caller]
    pub fn new(kind: McpTransportErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}
