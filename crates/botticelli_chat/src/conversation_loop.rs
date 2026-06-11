//! Conversation loop — stub pending Phase 7 migration to server-side orchestration.

use tracing::instrument;

/// Orchestrates multi-turn conversations with tool calling support.
///
/// Stub: Phase 7 will move this to the `botticelli_mcp` server as a server-side loop,
/// invoked by the client via tool calls through `botticelli_mcp_client::BotticelliClient`.
pub struct ConversationLoop;

impl ConversationLoop {
    /// Create a new conversation loop.
    #[instrument]
    pub fn new() -> Self {
        tracing::debug!("ConversationLoop stub created (Phase 7 will rewrite)");
        Self
    }
}

impl Default for ConversationLoop {
    #[instrument]
    fn default() -> Self {
        Self::new()
    }
}
