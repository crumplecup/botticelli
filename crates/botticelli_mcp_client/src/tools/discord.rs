//! Discord interaction tools.
//!
//! Exposes Discord bot capabilities as MCP tools for LLM orchestration.

use crate::{McpClientError, McpClientErrorKind, McpClientResult, ToolHandler};
use async_trait::async_trait;
use botticelli_social::BotticelliBot;
use pmcp::{Content, ToolInfo};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tool for sending Discord messages.
///
/// Sends text messages to specific Discord channels.
pub struct DiscordSendMessageTool {
    bot: Arc<Mutex<Option<BotticelliBot>>>,
}

impl DiscordSendMessageTool {
    /// Create new Discord send message tool.
    pub fn new(bot: Arc<Mutex<Option<BotticelliBot>>>) -> Self {
        Self { bot }
    }
}

#[async_trait]
impl ToolHandler for DiscordSendMessageTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "discord_send_message",
            Some("Send a message to a Discord channel. Returns message ID on success.".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "string",
                        "description": "Discord channel ID to send message to"
                    },
                    "content": {
                        "type": "string",
                        "description": "Message content to send"
                    }
                },
                "required": ["channel_id", "content"]
            }),
        )
    }

    #[tracing::instrument(skip(self, args), fields(tool = "discord_send_message"))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        tracing::debug!("Sending Discord message");

        let channel_id = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing channel_id parameter".to_string(),
                ))
            })?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing content parameter".to_string(),
                ))
            })?;

        let bot_guard = self.bot.lock().await;
        let bot = bot_guard.as_ref().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Discord bot not initialized".to_string(),
            ))
        })?;

        // Parse channel ID
        let channel_id_u64: u64 = channel_id.parse().map_err(|e| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(format!(
                "Invalid channel_id: {}",
                e
            )))
        })?;

        // Send message through repository
        match bot.repository().send_message(channel_id_u64, content).await {
            Ok(message_id) => {
                let result = json!({
                    "success": true,
                    "message_id": message_id.to_string(),
                    "channel_id": channel_id
                });

                tracing::debug!(message_id = %message_id, channel_id = %channel_id, "Message sent");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to send message");
                Err(McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    format!("Discord API error: {}", e),
                )))
            }
        }
    }
}

/// Tool for retrieving Discord messages.
///
/// Fetches messages from a specific Discord channel.
pub struct DiscordGetMessagesTool {
    bot: Arc<Mutex<Option<BotticelliBot>>>,
}

impl DiscordGetMessagesTool {
    /// Create new Discord get messages tool.
    pub fn new(bot: Arc<Mutex<Option<BotticelliBot>>>) -> Self {
        Self { bot }
    }
}

#[async_trait]
impl ToolHandler for DiscordGetMessagesTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "discord_get_messages",
            Some("Retrieve recent messages from a Discord channel.".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "string",
                        "description": "Discord channel ID to fetch messages from"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of messages to retrieve (default: 10, max: 100)"
                    }
                },
                "required": ["channel_id"]
            }),
        )
    }

    #[tracing::instrument(skip(self, args), fields(tool = "discord_get_messages"))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        tracing::debug!("Fetching Discord messages");

        let channel_id = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing channel_id parameter".to_string(),
                ))
            })?;

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(100) as u8;

        let bot_guard = self.bot.lock().await;
        let bot = bot_guard.as_ref().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Discord bot not initialized".to_string(),
            ))
        })?;

        // Parse channel ID
        let channel_id_u64: u64 = channel_id.parse().map_err(|e| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(format!(
                "Invalid channel_id: {}",
                e
            )))
        })?;

        // Fetch messages through repository
        match bot
            .repository()
            .get_messages(channel_id_u64, limit)
            .await
        {
            Ok(messages) => {
                let result = json!({
                    "success": true,
                    "channel_id": channel_id,
                    "count": messages.len(),
                    "messages": messages
                });

                tracing::debug!(count = messages.len(), channel_id = %channel_id, "Messages retrieved");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to fetch messages");
                Err(McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    format!("Discord API error: {}", e),
                )))
            }
        }
    }
}
