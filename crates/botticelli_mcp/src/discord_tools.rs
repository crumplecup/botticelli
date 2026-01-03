//! Discord integration tool types.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// Discord API Tools (discord.rs)
// ============================================================================

/// Parameters for posting a message to Discord.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for posting a message to a Discord channel")]
pub struct DiscordPostMessageParams {
    /// Discord channel ID.
    #[schemars(description = "Discord channel ID to post to")]
    pub channel_id: String,

    /// Message content (up to 2000 characters).
    #[schemars(description = "Message content (up to 2000 characters)")]
    pub content: String,
}

/// Result from posting a Discord message.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result from posting a Discord message")]
pub struct DiscordPostMessageResult {
    /// Status of the operation.
    #[schemars(description = "Status: 'success' or 'error'")]
    pub status: String,

    /// Created message ID.
    #[schemars(description = "ID of the created message")]
    pub message_id: String,

    /// Channel ID where message was posted.
    #[schemars(description = "Channel ID where message was posted")]
    pub channel_id: String,

    /// Timestamp of the message.
    #[schemars(description = "ISO 8601 timestamp of the message")]
    pub timestamp: String,
}

/// Parameters for getting Discord messages.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for fetching Discord message history")]
pub struct DiscordGetMessagesParams {
    /// Discord channel ID.
    #[schemars(description = "Discord channel ID to fetch from")]
    pub channel_id: String,

    /// Number of messages to fetch (1-100).
    #[serde(default = "default_message_limit")]
    #[schemars(
        description = "Number of messages to fetch (1-100, default: 50)",
        default = "default_message_limit"
    )]
    pub limit: i64,
}

fn default_message_limit() -> i64 {
    50
}

/// Discord author information.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Discord message author information")]
pub struct DiscordAuthor {
    /// User ID.
    #[schemars(description = "Discord user ID")]
    pub id: String,

    /// Username.
    #[schemars(description = "Discord username")]
    pub username: String,
}

/// Discord message information.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Discord message information")]
pub struct DiscordMessageInfo {
    /// Message ID.
    #[schemars(description = "Message ID")]
    pub id: String,

    /// Message content.
    #[schemars(description = "Message content")]
    pub content: String,

    /// Timestamp.
    #[schemars(description = "ISO 8601 timestamp")]
    pub timestamp: String,

    /// Author information.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Author information (if available)")]
    pub author: Option<DiscordAuthor>,
}

/// Result from getting Discord messages.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result from fetching Discord messages")]
pub struct DiscordGetMessagesResult {
    /// Status of the operation.
    #[schemars(description = "Status: 'success' or 'error'")]
    pub status: String,

    /// Channel ID.
    #[schemars(description = "Channel ID that was queried")]
    pub channel_id: String,

    /// Number of messages returned.
    #[schemars(description = "Number of messages returned")]
    pub count: usize,

    /// List of messages.
    #[schemars(description = "List of messages")]
    pub messages: Vec<DiscordMessageInfo>,
}

/// Parameters for getting Discord guild info.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for fetching Discord guild information")]
pub struct DiscordGetGuildInfoParams {
    /// Discord guild ID.
    #[schemars(description = "Discord guild (server) ID")]
    pub guild_id: String,
}

/// Result from getting Discord guild info.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result from fetching Discord guild information")]
pub struct DiscordGetGuildInfoResult {
    /// Status of the operation.
    #[schemars(description = "Status: 'success' or 'error'")]
    pub status: String,

    /// Guild ID.
    #[schemars(description = "Discord guild ID")]
    pub guild_id: String,

    /// Guild name.
    #[schemars(description = "Guild name")]
    pub name: String,

    /// Member count.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Number of members (if available)")]
    pub member_count: Option<u64>,
}

/// Parameters for getting Discord channels.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for listing Discord channels")]
pub struct DiscordGetChannelsParams {
    /// Discord guild ID.
    #[schemars(description = "Discord guild ID")]
    pub guild_id: String,
}

/// Discord channel information.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Discord channel information")]
pub struct DiscordChannelInfo {
    /// Channel ID.
    #[schemars(description = "Channel ID")]
    pub id: String,

    /// Channel name.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Channel name (if available)")]
    pub name: Option<String>,

    /// Channel type (0=text, 2=voice, 4=category, etc).
    #[schemars(description = "Channel type (0=text, 2=voice, 4=category, etc)")]
    #[serde(rename = "type")]
    pub channel_type: u8,
}

/// Result from getting Discord channels.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result from listing Discord channels")]
pub struct DiscordGetChannelsResult {
    /// Status of the operation.
    #[schemars(description = "Status: 'success' or 'error'")]
    pub status: String,

    /// Guild ID.
    #[schemars(description = "Guild ID that was queried")]
    pub guild_id: String,

    /// Number of channels.
    #[schemars(description = "Number of channels returned")]
    pub count: usize,

    /// List of channels.
    #[schemars(description = "List of channels")]
    pub channels: Vec<DiscordChannelInfo>,
}
