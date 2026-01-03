//! Chat service trait for UI/host decoupling

use async_trait::async_trait;

/// Chat service trait for handling LLM interactions with tools
#[async_trait]
pub trait ChatService: Send + Sync {
    /// Message type used in conversation history.
    type Message: Send + Sync;

    /// Tool definition type.
    type ToolDefinition: Send + Sync;

    /// Error type for operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Send a user message and get LLM response with tool execution
    async fn send_message(&mut self, user_input: String) -> Result<String, Self::Error>;

    /// Get available tools
    async fn get_tools(&self) -> Result<Vec<Self::ToolDefinition>, Self::Error>;

    /// Get conversation history
    fn get_history(&self) -> &[Self::Message];

    /// Clear conversation history
    fn clear_history(&mut self);
}
