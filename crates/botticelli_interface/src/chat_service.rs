//! Chat service trait for UI/host decoupling

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, Message, ToolDefinition};

/// Chat service trait for handling LLM interactions with tools
#[async_trait]
pub trait ChatService: Send + Sync {
    /// Send a user message and get LLM response with tool execution
    async fn send_message(
        &mut self,
        user_input: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Get available tools
    async fn get_tools(&self) -> Result<Vec<ToolDefinition>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get conversation history
    fn get_history(&self) -> &[Message];

    /// Clear conversation history
    fn clear_history(&mut self);
}
