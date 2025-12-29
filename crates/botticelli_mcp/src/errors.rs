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

impl From<ToolError> for rmcp::ErrorData {
    fn from(err: ToolError) -> Self {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;
        
        match err {
            ToolError::DialogNotConfigured => rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Dialog resource not configured",
                None,
            ),
            ToolError::DatabaseNotConfigured => rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Database operations not configured",
                None,
            ),
            ToolError::ElicitationFailed(msg) => rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Elicitation failed: {}", msg)),
                None,
            ),
            ToolError::InvalidInput(msg) => rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Invalid input: {}", msg)),
                None,
            ),
            ToolError::Internal(msg) => rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Internal error: {}", msg)),
                None,
            ),
        }
    }
}
