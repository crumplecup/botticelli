//! Discord bot actor for LLM-driven command handling.
//!
//! This module provides a Discord bot that responds to commands using
//! MCP orchestration for intelligent, context-aware responses.

use crate::DiscordMcpBridge;
use botticelli_mcp_client::Orchestrator;
use std::sync::Arc;
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
/// Receives Discord events, converts them to MCP tool calls,
/// and orchestrates LLM-driven responses.
#[derive(Clone, derive_getters::Getters)]
pub struct DiscordBot {
    /// MCP bridge for Discord events.
    bridge: DiscordMcpBridge,
}

impl DiscordBot {
    /// Creates a new Discord bot.
    #[tracing::instrument(skip(orchestrator))]
    pub fn new(orchestrator: Arc<Orchestrator>) -> Self {
        tracing::info!("Creating Discord bot");
        Self {
            bridge: DiscordMcpBridge::new(orchestrator),
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
                        "Processing Discord command"
                    );
                    
                    match self.bridge.handle_message(&channel_id, &user_id, &content).await {
                        Ok(response) => {
                            tracing::info!(
                                channel_id = %channel_id,
                                response_len = response.len(),
                                "Command processed successfully"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                channel_id = %channel_id,
                                "Failed to process command"
                            );
                        }
                    }
                }
                
                DiscordMessage::Shutdown => {
                    tracing::info!("Discord bot shutting down");
                    break;
                }
            }
        }
        
        tracing::info!("Discord bot stopped");
    }
}
