//! Bridge between Discord events and MCP orchestration.
//!
//! This module connects Discord bot events to the MCP orchestrator,
//! enabling LLM-driven responses to Discord interactions.

use botticelli_mcp_client::{McpClientError, Orchestrator};
use std::sync::Arc;

/// Bridge between Discord and MCP orchestration.
///
/// Converts Discord events into MCP tool calls and orchestrates
/// LLM-driven responses.
#[derive(Clone)]
pub struct DiscordMcpBridge {
    orchestrator: Arc<Orchestrator>,
}

/// Error type for Discord MCP bridge operations.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Discord MCP Bridge: {} at {}:{}", kind, file, line)]
pub struct DiscordMcpBridgeError {
    kind: DiscordMcpBridgeErrorKind,
    line: u32,
    file: &'static str,
}

/// Specific error conditions for Discord MCP bridge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum DiscordMcpBridgeErrorKind {
    /// MCP orchestrator error occurred.
    #[display("MCP orchestrator error: {}", _0)]
    Orchestrator(String),

    /// Invalid message format.
    #[display("Invalid message format: {}", _0)]
    InvalidFormat(String),

    /// Missing required field in response.
    #[display("Missing required field: {}", _0)]
    MissingField(String),
}

impl DiscordMcpBridgeError {
    #[track_caller]
    fn new(kind: DiscordMcpBridgeErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

impl From<McpClientError> for DiscordMcpBridgeError {
    fn from(err: McpClientError) -> Self {
        Self::new(DiscordMcpBridgeErrorKind::Orchestrator(err.to_string()))
    }
}

impl DiscordMcpBridge {
    /// Create a new Discord MCP bridge.
    pub fn new(orchestrator: Arc<Orchestrator>) -> Self {
        Self { orchestrator }
    }

    /// Process a Discord message and get response.
    #[tracing::instrument(skip(self))]
    pub async fn handle_message(
        &self,
        channel_id: &str,
        user_id: &str,
        content: &str,
    ) -> Result<String, DiscordMcpBridgeError> {
        use botticelli_mcp_client::{Message, MessageRole};

        tracing::debug!(
            channel_id = %channel_id,
            user_id = %user_id,
            "Processing Discord message through MCP orchestrator"
        );

        // Create initial message from user input
        let messages = vec![Message::new(
            MessageRole::User,
            content.to_string(),
            vec![], // No tool calls in initial user message
            vec![], // No tool results in initial user message
        )];

        // Execute agentic loop through orchestrator
        let response = self
            .orchestrator
            .execute(messages)
            .await
            .map_err(DiscordMcpBridgeError::from)?;

        tracing::info!(
            channel_id = %channel_id,
            response_len = response.len(),
            "Generated response through MCP orchestrator"
        );

        Ok(response)
    }
}
