//! Chat host trait definition for MCP-based chat applications.

/// Trait for managing chat conversations with MCP tool integration.
///
/// This trait provides the complete interface needed for a chat UI:
/// - Sending user messages and receiving LLM responses
/// - Accessing conversation history
/// - Tool discovery and execution
///
/// Implementors should provide full instrumentation for observability.
#[async_trait::async_trait]
pub trait ChatHost: Send + Sync {
    /// Chat message type for conversation history.
    type ChatMessage: Send + Sync + Clone;

    /// Tool definition type.
    type ToolDefinition: Send + Sync;

    /// Error type for operations.
    type Error: std::error::Error + Send + Sync + 'static;

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
    async fn send_message(&mut self, user_message: String) -> Result<String, Self::Error>;

    /// Get the full conversation history.
    ///
    /// Returns all messages in chronological order.
    ///
    /// # Errors
    ///
    /// Returns error if conversation cannot be retrieved.
    async fn get_conversation(&self) -> Result<Vec<Self::ChatMessage>, Self::Error>;

    /// Get all available tools from connected MCP servers.
    ///
    /// # Errors
    ///
    /// Returns error if tools cannot be retrieved.
    async fn available_tools(&self) -> Result<Vec<Self::ToolDefinition>, Self::Error>;

    /// Execute a tool by name with given arguments.
    ///
    /// # Errors
    ///
    /// Returns error if tool execution fails.
    async fn execute_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, Self::Error>;

    /// Check if any MCP servers are connected.
    async fn has_tools(&self) -> bool {
        self.available_tools()
            .await
            .map(|tools| !tools.is_empty())
            .unwrap_or(false)
    }
}
