//! Tool execution result type.

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Result from executing a tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Getters)]
pub struct ToolResult {
    /// ID of the tool call this responds to
    tool_call_id: String,
    /// Output from the tool execution (as JSON)
    content: Value,
    /// Whether the tool execution failed
    is_error: bool,
}

impl ToolResult {
    /// Creates a new tool result.
    #[must_use]
    pub fn new(tool_call_id: String, content: Value, is_error: bool) -> Self {
        Self {
            tool_call_id,
            content,
            is_error,
        }
    }
}
