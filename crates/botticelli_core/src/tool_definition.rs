//! Tool definition types for LLM tool calling.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP-compliant tool definition.
///
/// Describes a tool that can be called by the LLM during generation.
/// Follows the Model Context Protocol specification for tool definitions.
///
/// # Examples
///
/// ```
/// use botticelli_core::ToolDefinition;
/// use serde_json::json;
///
/// let tool = ToolDefinition::new(
///     "get_weather".to_string(),
///     "Get the current weather for a location".to_string(),
///     json!({
///         "type": "object",
///         "properties": {
///             "location": {
///                 "type": "string",
///                 "description": "City name"
///             }
///         },
///         "required": ["location"]
///     }),
/// );
///
/// assert_eq!(tool.name(), "get_weather");
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_setters::Setters,
    derive_new::new,
    elicitation::Elicit,
)]
#[setters(prefix = "with_")]
pub struct ToolDefinition {
    /// Tool name (must be unique in the registry)
    name: String,
    /// Human-readable description
    description: String,
    /// JSON Schema for tool input parameters
    input_schema: Value,
}
