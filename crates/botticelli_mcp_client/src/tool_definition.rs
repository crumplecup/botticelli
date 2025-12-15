//! Tool definition types.

use serde_json::Value;

/// Represents a tool definition for LLM context.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON schema for tool parameters.
    pub input_schema: Value,
}
