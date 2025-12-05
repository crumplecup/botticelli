//! Processors for Discord data extraction and storage.

use crate::{ActProcessor, ProcessorContext};
use async_trait::async_trait;
use botticelli_database::PgPool;
use botticelli_error::BotticelliResult;
use botticelli_social::{DiscordChannel, DiscordGuild};
use serde_json::Value as JsonValue;

/// Processor that extracts Discord guild data from act responses and stores in database.
///
/// This processor looks for JSON responses containing Discord guild information
/// and stores them in the `discord_guilds` table.
pub struct DiscordGuildProcessor {
    pool: PgPool,
}

impl DiscordGuildProcessor {
    /// Create a new Discord guild processor with the given database pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Extract guild data from JSON response.
    fn extract_guild(json: &JsonValue) -> Option<DiscordGuild> {
        // Try to parse as a single guild
        if let Ok(guild) = serde_json::from_value::<DiscordGuild>(json.clone()) {
            return Some(guild);
        }

        // Try to extract from common response structures
        if let Some(guild_data) = json.get("guild") {
            if let Ok(guild) = serde_json::from_value::<DiscordGuild>(guild_data.clone()) {
                return Some(guild);
            }
        }

        None
    }
}

#[async_trait]
impl ActProcessor for DiscordGuildProcessor {
    #[tracing::instrument(skip(self, context), fields(act = %context.execution.act_name))]
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        tracing::debug!("Processing Discord guild data");

        // Parse response as JSON
        let json: JsonValue = serde_json::from_str(&context.execution.response)
            .map_err(|e| botticelli_error::BackendError::new(format!("Failed to parse JSON: {}", e)))?;

        // Extract guild
        let guild = Self::extract_guild(&json)
            .ok_or_else(|| botticelli_error::BackendError::new("No guild data found in response"))?;

        // Store in database
        let mut conn = self.pool.get()
            .map_err(|e| botticelli_error::BackendError::new(format!("Failed to get database connection: {}", e)))?;

        guild.insert(&mut conn)
            .map_err(|e| botticelli_error::BackendError::new(format!("Failed to insert guild: {}", e)))?;

        tracing::info!(guild_id = %guild.id, guild_name = %guild.name, "Stored Discord guild");

        Ok(())
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        // Process if response looks like JSON and contains guild-like data
        if let Ok(json) = serde_json::from_str::<JsonValue>(&context.execution.response) {
            // Check for guild indicators
            if json.get("id").is_some() && json.get("name").is_some() {
                return true;
            }
            if json.get("guild").is_some() {
                return true;
            }
        }
        false
    }

    fn name(&self) -> &str {
        "DiscordGuildProcessor"
    }
}

/// Processor that extracts Discord channel data from act responses and stores in database.
///
/// This processor looks for JSON responses containing Discord channel information
/// and stores them in the `discord_channels` table.
pub struct DiscordChannelProcessor {
    pool: PgPool,
}

impl DiscordChannelProcessor {
    /// Create a new Discord channel processor with the given database pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Extract channels from JSON response.
    fn extract_channels(json: &JsonValue) -> Vec<DiscordChannel> {
        let mut channels = Vec::new();

        // Try to parse as a single channel
        if let Ok(channel) = serde_json::from_value::<DiscordChannel>(json.clone()) {
            channels.push(channel);
            return channels;
        }

        // Try to parse as array of channels
        if let Ok(channel_list) = serde_json::from_value::<Vec<DiscordChannel>>(json.clone()) {
            return channel_list;
        }

        // Try to extract from common response structures
        if let Some(channels_data) = json.get("channels") {
            if let Ok(channel_list) = serde_json::from_value::<Vec<DiscordChannel>>(channels_data.clone()) {
                return channel_list;
            }
        }

        channels
    }
}

#[async_trait]
impl ActProcessor for DiscordChannelProcessor {
    #[tracing::instrument(skip(self, context), fields(act = %context.execution.act_name))]
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        tracing::debug!("Processing Discord channel data");

        // Parse response as JSON
        let json: JsonValue = serde_json::from_str(&context.execution.response)
            .map_err(|e| botticelli_error::BackendError::new(format!("Failed to parse JSON: {}", e)))?;

        // Extract channels
        let channels = Self::extract_channels(&json);
        if channels.is_empty() {
            return Err(botticelli_error::BackendError::new("No channel data found in response").into());
        }

        // Store in database
        let mut conn = self.pool.get()
            .map_err(|e| botticelli_error::BackendError::new(format!("Failed to get database connection: {}", e)))?;

        for channel in &channels {
            channel.insert(&mut conn)
                .map_err(|e| botticelli_error::BackendError::new(format!("Failed to insert channel: {}", e)))?;
        }

        tracing::info!(channel_count = channels.len(), "Stored Discord channels");

        Ok(())
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        // Process if response looks like JSON and contains channel-like data
        if let Ok(json) = serde_json::from_str::<JsonValue>(&context.execution.response) {
            // Check for channel indicators
            if json.get("id").is_some() && json.get("type").is_some() {
                return true;
            }
            if json.get("channels").is_some() {
                return true;
            }
            if json.is_array() {
                if let Some(first) = json.as_array().and_then(|a| a.first()) {
                    if first.get("id").is_some() && first.get("type").is_some() {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn name(&self) -> &str {
        "DiscordChannelProcessor"
    }
}
