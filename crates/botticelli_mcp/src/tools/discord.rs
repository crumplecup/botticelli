//! Discord API tools for MCP.
//!
//! These tools provide direct access to Discord's HTTP API, enabling LLMs
//! to post messages, query channels, and interact with Discord servers.

#[cfg(feature = "discord")]
use crate::tools::McpTool;

#[cfg(feature = "discord")]
use crate::{McpError, McpResult};

#[cfg(feature = "discord")]
use async_trait::async_trait;

#[cfg(feature = "discord")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "discord")]
use serde_json::{json, Value};

#[cfg(feature = "discord")]
use reqwest::Client;

#[cfg(feature = "discord")]
use std::sync::Arc;

#[cfg(feature = "discord")]
use tracing::{debug, instrument};

/// Discord API base URL.
#[cfg(feature = "discord")]
const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

/// Discord message response from API.
#[cfg(feature = "discord")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscordMessage {
    id: String,
    channel_id: String,
    content: String,
    timestamp: String,
    #[serde(default)]
    author: Option<DiscordUser>,
}

/// Discord user information.
#[cfg(feature = "discord")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    #[serde(default)]
    discriminator: String,
}

/// Discord channel information.
#[cfg(feature = "discord")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscordChannel {
    id: String,
    #[serde(rename = "type")]
    channel_type: u8,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    guild_id: Option<String>,
}

/// Discord guild information.
#[cfg(feature = "discord")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscordGuild {
    id: String,
    name: String,
    #[serde(default)]
    member_count: Option<u64>,
}

/// Shared Discord HTTP client for all tools.
#[cfg(feature = "discord")]
#[derive(Clone)]
struct DiscordClient {
    client: Client,
    token: String,
}

#[cfg(feature = "discord")]
impl DiscordClient {
    /// Creates a new Discord HTTP client.
    fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    /// Makes an authenticated GET request to Discord API.
    #[instrument(skip(self), fields(endpoint))]
    async fn get(&self, endpoint: &str) -> McpResult<Value> {
        let url = format!("{}{}", DISCORD_API_BASE, endpoint);
        debug!(url = %url, "Discord API GET");

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .header("User-Agent", "Botticelli-MCP/0.1.0")
            .send()
            .await
            .map_err(|e| {
                McpError::ToolExecutionFailed(format!("Discord API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::ToolExecutionFailed(format!(
                "Discord API error {}: {}",
                status, body
            )));
        }

        response.json().await.map_err(|e| {
            McpError::ToolExecutionFailed(format!("Failed to parse Discord response: {}", e))
        })
    }

    /// Makes an authenticated POST request to Discord API.
    #[instrument(skip(self, body), fields(endpoint))]
    async fn post(&self, endpoint: &str, body: Value) -> McpResult<Value> {
        let url = format!("{}{}", DISCORD_API_BASE, endpoint);
        debug!(url = %url, "Discord API POST");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .header("User-Agent", "Botticelli-MCP/0.1.0")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                McpError::ToolExecutionFailed(format!("Discord API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::ToolExecutionFailed(format!(
                "Discord API error {}: {}",
                status, body
            )));
        }

        response.json().await.map_err(|e| {
            McpError::ToolExecutionFailed(format!("Failed to parse Discord response: {}", e))
        })
    }
}

// ============================================================================
// Tool: Post Message
// ============================================================================

/// Tool for posting messages to Discord channels.
#[cfg(feature = "discord")]
pub struct DiscordPostMessageTool {
    client: Arc<DiscordClient>,
}

#[cfg(feature = "discord")]
impl DiscordPostMessageTool {
    /// Creates a new Discord post message tool.
    pub fn new() -> Result<Self, String> {
        let token = std::env::var("DISCORD_TOKEN")
            .map_err(|_| "DISCORD_TOKEN environment variable not set".to_string())?;
        Ok(Self {
            client: Arc::new(DiscordClient::new(token)),
        })
    }
}

#[cfg(feature = "discord")]
#[async_trait]
impl McpTool for DiscordPostMessageTool {
    fn name(&self) -> &str {
        "discord_post_message"
    }

    fn description(&self) -> &str {
        "Post a message to a Discord channel. Requires DISCORD_TOKEN environment variable."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "channel_id": {
                    "type": "string",
                    "description": "Discord channel ID to post to"
                },
                "content": {
                    "type": "string",
                    "description": "Message content (up to 2000 characters)"
                }
            },
            "required": ["channel_id", "content"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let channel_id = input
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'channel_id'".to_string()))?;

        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'content'".to_string()))?;

        if content.len() > 2000 {
            return Err(McpError::InvalidInput(
                "Content exceeds 2000 character limit".to_string(),
            ));
        }

        let body = json!({ "content": content });
        let response = self
            .client
            .post(&format!("/channels/{}/messages", channel_id), body)
            .await?;

        let message: DiscordMessage = serde_json::from_value(response)
            .map_err(|e| McpError::ToolExecutionFailed(format!("Failed to parse message: {}", e)))?;

        Ok(json!({
            "status": "success",
            "message_id": message.id,
            "channel_id": message.channel_id,
            "timestamp": message.timestamp,
        }))
    }
}

// ============================================================================
// Tool: Get Messages
// ============================================================================

/// Tool for fetching message history from Discord channels.
#[cfg(feature = "discord")]
pub struct DiscordGetMessagesTool {
    client: Arc<DiscordClient>,
}

#[cfg(feature = "discord")]
impl DiscordGetMessagesTool {
    /// Creates a new Discord get messages tool.
    pub fn new() -> Result<Self, String> {
        let token = std::env::var("DISCORD_TOKEN")
            .map_err(|_| "DISCORD_TOKEN environment variable not set".to_string())?;
        Ok(Self {
            client: Arc::new(DiscordClient::new(token)),
        })
    }
}

#[cfg(feature = "discord")]
#[async_trait]
impl McpTool for DiscordGetMessagesTool {
    fn name(&self) -> &str {
        "discord_get_messages"
    }

    fn description(&self) -> &str {
        "Fetch message history from a Discord channel. Requires DISCORD_TOKEN environment variable."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "channel_id": {
                    "type": "string",
                    "description": "Discord channel ID to fetch from"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of messages to fetch (1-100)",
                    "default": 50,
                    "minimum": 1,
                    "maximum": 100
                }
            },
            "required": ["channel_id"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let channel_id = input
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'channel_id'".to_string()))?;

        let limit = input
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(50)
            .clamp(1, 100);

        let response = self
            .client
            .get(&format!("/channels/{}/messages?limit={}", channel_id, limit))
            .await?;

        let messages: Vec<DiscordMessage> = serde_json::from_value(response)
            .map_err(|e| McpError::ToolExecutionFailed(format!("Failed to parse messages: {}", e)))?;

        let formatted_messages: Vec<Value> = messages
            .into_iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "content": m.content,
                    "timestamp": m.timestamp,
                    "author": m.author.as_ref().map(|a| json!({
                        "id": a.id,
                        "username": a.username,
                    })),
                })
            })
            .collect();

        Ok(json!({
            "status": "success",
            "channel_id": channel_id,
            "count": formatted_messages.len(),
            "messages": formatted_messages,
        }))
    }
}

// ============================================================================
// Tool: Get Guild Info
// ============================================================================

/// Tool for fetching Discord guild (server) information.
#[cfg(feature = "discord")]
pub struct DiscordGetGuildInfoTool {
    client: Arc<DiscordClient>,
}

#[cfg(feature = "discord")]
impl DiscordGetGuildInfoTool {
    /// Creates a new Discord get guild info tool.
    pub fn new() -> Result<Self, String> {
        let token = std::env::var("DISCORD_TOKEN")
            .map_err(|_| "DISCORD_TOKEN environment variable not set".to_string())?;
        Ok(Self {
            client: Arc::new(DiscordClient::new(token)),
        })
    }
}

#[cfg(feature = "discord")]
#[async_trait]
impl McpTool for DiscordGetGuildInfoTool {
    fn name(&self) -> &str {
        "discord_get_guild_info"
    }

    fn description(&self) -> &str {
        "Get information about a Discord guild (server). Requires DISCORD_TOKEN environment variable."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "guild_id": {
                    "type": "string",
                    "description": "Discord guild ID"
                }
            },
            "required": ["guild_id"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let guild_id = input
            .get("guild_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'guild_id'".to_string()))?;

        let response = self
            .client
            .get(&format!("/guilds/{}?with_counts=true", guild_id))
            .await?;

        let guild: DiscordGuild = serde_json::from_value(response)
            .map_err(|e| McpError::ToolExecutionFailed(format!("Failed to parse guild: {}", e)))?;

        Ok(json!({
            "status": "success",
            "guild_id": guild.id,
            "name": guild.name,
            "member_count": guild.member_count,
        }))
    }
}

// ============================================================================
// Tool: Get Channels
// ============================================================================

/// Tool for listing channels in a Discord guild.
#[cfg(feature = "discord")]
pub struct DiscordGetChannelsTool {
    client: Arc<DiscordClient>,
}

#[cfg(feature = "discord")]
impl DiscordGetChannelsTool {
    /// Creates a new Discord get channels tool.
    pub fn new() -> Result<Self, String> {
        let token = std::env::var("DISCORD_TOKEN")
            .map_err(|_| "DISCORD_TOKEN environment variable not set".to_string())?;
        Ok(Self {
            client: Arc::new(DiscordClient::new(token)),
        })
    }
}

#[cfg(feature = "discord")]
#[async_trait]
impl McpTool for DiscordGetChannelsTool {
    fn name(&self) -> &str {
        "discord_get_channels"
    }

    fn description(&self) -> &str {
        "List channels in a Discord guild. Requires DISCORD_TOKEN environment variable."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "guild_id": {
                    "type": "string",
                    "description": "Discord guild ID"
                }
            },
            "required": ["guild_id"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let guild_id = input
            .get("guild_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'guild_id'".to_string()))?;

        let response = self
            .client
            .get(&format!("/guilds/{}/channels", guild_id))
            .await?;

        let channels: Vec<DiscordChannel> = serde_json::from_value(response)
            .map_err(|e| McpError::ToolExecutionFailed(format!("Failed to parse channels: {}", e)))?;

        let formatted_channels: Vec<Value> = channels
            .into_iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "name": c.name,
                    "type": c.channel_type,
                })
            })
            .collect();

        Ok(json!({
            "status": "success",
            "guild_id": guild_id,
            "count": formatted_channels.len(),
            "channels": formatted_channels,
        }))
    }
}
