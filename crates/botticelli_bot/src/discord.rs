//! Discord bot actor — stub pending Phase 7 rewrite to use BotticelliClient.

use crate::DiscordMcpBridge;
use tokio::sync::mpsc;

/// Messages for the Discord bot actor.
#[derive(Debug)]
pub enum DiscordMessage {
    /// Process a Discord command with LLM orchestration.
    ProcessCommand {
        /// Channel ID where command was received.
        channel_id: String,
        /// User ID who sent the command.
        user_id: String,
        /// Command content.
        content: String,
    },

    /// Shutdown the bot.
    Shutdown,
}

/// Discord bot actor for handling commands via MCP orchestration.
///
/// Stub: Phase 7 will rewrite this to use `botticelli_mcp_client::BotticelliClient`.
#[derive(Clone, derive_getters::Getters)]
pub struct DiscordBot {
    /// MCP bridge for Discord events.
    bridge: DiscordMcpBridge,
}

impl DiscordBot {
    /// Creates a new Discord bot.
    #[tracing::instrument]
    pub fn new() -> Self {
        tracing::info!("Creating Discord bot (stub)");
        Self {
            bridge: DiscordMcpBridge,
        }
    }

    /// Runs the Discord bot actor.
    #[tracing::instrument(skip(self, rx))]
    pub async fn run(self, mut rx: mpsc::Receiver<DiscordMessage>) {
        tracing::info!("Discord bot starting");
        while let Some(msg) = rx.recv().await {
            match msg {
                DiscordMessage::ProcessCommand {
                    channel_id,
                    user_id,
                    content,
                } => {
                    tracing::debug!(
                        channel_id = %channel_id,
                        user_id = %user_id,
                        content_len = content.len(),
                        "Received command (stub — not yet implemented)"
                    );
                }
                DiscordMessage::Shutdown => {
                    tracing::info!("Discord bot shutting down");
                    break;
                }
            }
        }
    }
}

impl Default for DiscordBot {
    #[tracing::instrument]
    fn default() -> Self {
        Self::new()
    }
}
