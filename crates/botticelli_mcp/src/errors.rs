//! Error types for MCP tools.
//!
//! All errors use derive_more for Display and Error implementations,
//! following CLAUDE.md standards.

use derive_more::{Display, Error};

/// Errors that can occur during tool execution.
#[derive(Debug, Clone, Display, Error)]
pub enum ToolError {
    /// Dialog resource not configured.
    #[display("Dialog resource not configured for this server")]
    DialogNotConfigured,
    
    /// Database operations not configured.
    #[display("Database operations not configured for this server")]
    DatabaseNotConfigured,
    
    /// Elicitation failed.
    #[display("Elicitation failed: {}", _0)]
    ElicitationFailed(#[error(not(source))] String),
    
    /// Invalid input provided.
    #[display("Invalid input: {}", _0)]
    InvalidInput(#[error(not(source))] String),
    
    /// Internal error occurred.
    #[display("Internal error: {}", _0)]
    Internal(#[error(not(source))] String),
}

impl From<botticelli_error::McpError> for ToolError {
    fn from(err: botticelli_error::McpError) -> Self {
        Self::Internal(err.to_string())
    }
}
