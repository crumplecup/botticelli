//! Discord integration tools.
//!
//! Tools for interacting with Discord servers, channels, and messages.

use crate::rmcp_server::helpers::to_mcp_error;
use crate::rmcp_server::BotticelliServer;
use crate::{
    DiscordAuthor, DiscordChannelInfo, DiscordGetChannelsParams, DiscordGetChannelsResult,
    DiscordGetGuildInfoParams, DiscordGetGuildInfoResult, DiscordGetMessagesParams,
    DiscordGetMessagesResult, DiscordMessageInfo, DiscordPostMessageParams,
    DiscordPostMessageResult,
};
use rmcp::handler::server::wrapper::{Json, Parameters};
use tracing::{debug, instrument};

impl BotticelliServer {
    /// Post a message to a Discord channel.
    #[instrument(skip(self, params), fields(channel_id = params.channel_id(), content_len = params.content().len()))]
    pub async fn discord_post_message(
        &self,
        Parameters(params): Parameters<DiscordPostMessageParams>,
    ) -> Result<Json<DiscordPostMessageResult>, rmcp::ErrorData> {
        use reqwest::Client;
        use rmcp::model::ErrorCode;
        use serde_json::json;
        use std::borrow::Cow;

        let channel_id = params.channel_id().clone();
        let content = params.content().clone();

        debug!(
            channel_id,
            content_len = content.len(),
            "Posting Discord message"
        );

        // Validate content length
        if content.len() > 2000 {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("Content exceeds 2000 character limit"),
                None,
            ));
        }

        // Get Discord token
        let token = std::env::var("DISCORD_TOKEN").map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("DISCORD_TOKEN environment variable not set"),
                None,
            )
        })?;

        // Make Discord API request
        let client = Client::new();
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            channel_id
        );

        let response = client
            .post(&url)
            .header("Authorization", format!("Bot {}", token))
            .header("User-Agent", "Botticelli-MCP/0.2.0")
            .header("Content-Type", "application/json")
            .json(&json!({ "content": content }))
            .send()
            .await
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Discord API request failed: {}", e)),
                    None,
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Discord API error {}: {}", status, body)),
                None,
            ));
        }

        let message: serde_json::Value = response
            .json()
            .await
            .map_err(|e| to_mcp_error(e, "Failed to parse Discord response"))?;

        let message_id = message["id"].as_str().unwrap_or("unknown").to_string();
        let timestamp = message["timestamp"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        debug!(message_id, "Discord message posted successfully");

        Ok(Json(DiscordPostMessageResult::new(
            "success".to_string(),
            message_id,
            channel_id,
            timestamp,
        )))
    }
    
    /// Get recent messages from a Discord channel.
    #[instrument(skip(self, params), fields(channel_id = params.channel_id(), limit = params.limit()))]
    pub async fn discord_get_messages(
        &self,
        Parameters(params): Parameters<DiscordGetMessagesParams>,
    ) -> Result<Json<DiscordGetMessagesResult>, rmcp::ErrorData> {
        use reqwest::Client;
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        let channel_id = params.channel_id().clone();
        let limit = (*params.limit()).clamp(1, 100);
        debug!(channel_id, limit, "Getting Discord messages");

        // Get Discord token
        let token = std::env::var("DISCORD_TOKEN").map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("DISCORD_TOKEN environment variable not set"),
                None,
            )
        })?;

        // Make Discord API request
        let client = Client::new();
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages?limit={}",
            channel_id, limit
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bot {}", token))
            .header("User-Agent", "Botticelli-MCP/0.2.0")
            .send()
            .await
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Discord API request failed: {}", e)),
                    None,
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Discord API error {}: {}", status, body)),
                None,
            ));
        }

        let messages: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| to_mcp_error(e, "Failed to parse Discord response"))?;

        let formatted_messages: Vec<DiscordMessageInfo> = messages
            .into_iter()
            .map(|m| {
                DiscordMessageInfo::new(
                    m["id"].as_str().unwrap_or("").to_string(),
                    m["content"].as_str().unwrap_or("").to_string(),
                    m["timestamp"].as_str().unwrap_or("").to_string(),
                    m["author"].as_object().map(|a| {
                        DiscordAuthor::new(
                            a["id"].as_str().unwrap_or("").to_string(),
                            a["username"].as_str().unwrap_or("").to_string(),
                        )
                    }),
                )
            })
            .collect();

        debug!(
            count = formatted_messages.len(),
            "Discord messages retrieved"
        );

        Ok(Json(DiscordGetMessagesResult::new(
            "success".to_string(),
            channel_id,
            formatted_messages.len(),
            formatted_messages,
        )))
    }
    
    /// Get information about a Discord guild (server).
    #[instrument(skip(self, params), fields(guild_id = params.guild_id()))]
    pub async fn discord_get_guild_info(
        &self,
        Parameters(params): Parameters<DiscordGetGuildInfoParams>,
    ) -> Result<Json<DiscordGetGuildInfoResult>, rmcp::ErrorData> {
        use reqwest::Client;
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        let guild_id = params.guild_id().clone();
        debug!(guild_id, "Getting Discord guild info");

        // Get Discord token
        let token = std::env::var("DISCORD_TOKEN").map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("DISCORD_TOKEN environment variable not set"),
                None,
            )
        })?;

        // Make Discord API request
        let client = Client::new();
        let url = format!(
            "https://discord.com/api/v10/guilds/{}?with_counts=true",
            guild_id
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bot {}", token))
            .header("User-Agent", "Botticelli-MCP/0.2.0")
            .send()
            .await
            .map_err(|e| to_mcp_error(e, "Discord API request failed"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Discord API error {}: {}", status, body)),
                None,
            ));
        }

        let guild: serde_json::Value = response
            .json()
            .await
            .map_err(|e| to_mcp_error(e, "Failed to parse Discord response"))?;

        let name = guild["name"].as_str().unwrap_or("Unknown").to_string();
        let member_count = guild["approximate_member_count"].as_u64();

        debug!(guild_id, name, "Discord guild info retrieved");

        Ok(Json(DiscordGetGuildInfoResult::new(
            "success".to_string(),
            guild_id,
            name,
            member_count,
        )))
    }

    /// List channels in a Discord guild.
    #[instrument(skip(self, params), fields(guild_id = params.guild_id()))]
    pub async fn discord_get_channels(
        &self,
        Parameters(params): Parameters<DiscordGetChannelsParams>,
    ) -> Result<Json<DiscordGetChannelsResult>, rmcp::ErrorData> {
        use reqwest::Client;
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        let guild_id = params.guild_id().clone();
        debug!(guild_id, "Getting Discord channels");

        // Get Discord token
        let token = std::env::var("DISCORD_TOKEN").map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("DISCORD_TOKEN environment variable not set"),
                None,
            )
        })?;

        // Make Discord API request
        let client = Client::new();
        let url = format!("https://discord.com/api/v10/guilds/{}/channels", guild_id);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bot {}", token))
            .header("User-Agent", "Botticelli-MCP/0.2.0")
            .send()
            .await
            .map_err(|e| to_mcp_error(e, "Discord API request failed"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Discord API error {}: {}", status, body)),
                None,
            ));
        }

        let channels: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| to_mcp_error(e, "Failed to parse Discord response"))?;

        let formatted_channels: Vec<DiscordChannelInfo> = channels
            .into_iter()
            .map(|c| {
                DiscordChannelInfo::new(
                    c["id"].as_str().unwrap_or("").to_string(),
                    c["name"].as_str().map(|s| s.to_string()),
                    c["type"].as_u64().unwrap_or(0) as u8,
                )
            })
            .collect();

        debug!(
            count = formatted_channels.len(),
            "Discord channels retrieved"
        );

        Ok(Json(DiscordGetChannelsResult::new(
            "success".to_string(),
            guild_id,
            formatted_channels.len(),
            formatted_channels,
        )))
    }
}

