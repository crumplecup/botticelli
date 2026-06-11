//! MCP chat host — stub pending Phase 7 migration to server-side orchestration.

use botticelli_core::ToolDefinition;
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use botticelli_interface::{ChatHost, ChatMessage};
use tracing::instrument;

/// Chat host that delegates to the Botticelli MCP server.
///
/// Stub: Phase 7 will rewrite this to use `botticelli_mcp_client::BotticelliClient`.
pub struct McpChatHost;

impl McpChatHost {
    /// Create a new MCP chat host.
    #[instrument]
    pub fn new() -> Self {
        tracing::debug!("McpChatHost stub created (Phase 7 will rewrite)");
        Self
    }
}

impl Default for McpChatHost {
    #[instrument]
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ChatHost for McpChatHost {
    #[instrument(skip(self))]
    async fn send_message(&mut self, _user_message: String) -> ChatResult<String> {
        Err(ChatError::new(ChatErrorKind::NotImplemented(
            "McpChatHost stub — Phase 7 will implement".to_string(),
        )))
    }

    #[instrument(skip(self))]
    async fn get_conversation(&self) -> ChatResult<Vec<ChatMessage>> {
        Ok(Vec::new())
    }

    #[instrument(skip(self))]
    async fn available_tools(&self) -> ChatResult<Vec<ToolDefinition>> {
        Ok(Vec::new())
    }

    #[instrument(skip(self))]
    async fn execute_tool(
        &mut self,
        _name: &str,
        _arguments: serde_json::Value,
    ) -> ChatResult<serde_json::Value> {
        Err(ChatError::new(ChatErrorKind::NotImplemented(
            "McpChatHost stub — Phase 7 will implement".to_string(),
        )))
    }
}
