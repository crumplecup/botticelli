//! Tool execution result type.

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Result from executing a tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Getters, derive_new::new, elicitation::Elicit)]
pub struct ToolResult {
    /// ID of the tool call this responds to
    tool_call_id: String,
    /// Output from the tool execution (as JSON)
    content: Value,
    /// Whether the tool execution failed
    is_error: bool,
}
