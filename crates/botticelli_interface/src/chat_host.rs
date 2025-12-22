//! Chat host trait definition for MCP-based chat applications.

use botticelli_core::ToolDefinition;
use botticelli_error::ChatResult;

/// A chat message that can be displayed in the UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    /// Role: "user", "assistant", or "system"
    pub role: String,
    /// Message content
    pub content: String,
}

impl ChatMessage {
    /// Creates a user message.
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
        }
    }

    /// Creates an assistant message.
    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
        }
    }

    /// Creates a system message.
    pub fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content,
        }
    }
}

/// Trait for managing chat conversations with MCP tool integration.
///
/// This trait provides the complete interface needed for a chat UI:
/// - Sending user messages and receiving LLM responses
/// - Accessing conversation history
/// - Tool discovery and execution
///
/// Implementors should provide full instrumentation for observability.
pub trait ChatHost: Send + Sync {
    /// Send a user message and get the assistant's response.
    ///
    /// This method handles the complete conversation turn:
    /// 1. Add user message to conversation
    /// 2. Send to LLM with available tools
    /// 3. Handle any tool calls in response
    /// 4. Return final assistant response
    ///
    /// # Errors
    ///
    /// Returns error if message cannot be processed or LLM call fails.
    fn send_message(&mut self, user_message: String) -> ChatResult<String>;

    /// Get the full conversation history.
    ///
    /// Returns all messages in chronological order.
    ///
    /// # Errors
    ///
    /// Returns error if conversation cannot be retrieved.
    fn get_conversation(&self) -> ChatResult<Vec<ChatMessage>>;

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
