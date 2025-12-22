//! Chat host trait definition for MCP-based chat applications.

use botticelli_core::ToolDefinition;
use botticelli_error::ChatResult;

/// Trait for managing MCP tool integrations in chat applications.
///
/// Implementors provide access to available tools and their execution.
/// This separates the tool management layer from specific UI implementations.
pub trait ChatHost {
    /// Get all available tools from connected MCP servers.
    ///
    /// # Errors
    ///
    /// Returns error if tools cannot be retrieved.
    fn available_tools(&self) -> ChatResult<Vec<ToolDefinition>>;

    /// Execute a tool by name with given arguments.
    ///
    /// # Errors
    ///
    /// Returns error if tool execution fails.
    fn execute_tool(&mut self, name: &str, arguments: serde_json::Value) -> ChatResult<serde_json::Value>;

    /// Check if any MCP servers are connected.
    fn has_tools(&self) -> bool {
        self.available_tools().map(|tools| !tools.is_empty()).unwrap_or(false)
    }
}
