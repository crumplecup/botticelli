//! Discord bot command executor.
//!
//! This module provides the Discord implementation of the BotCommandExecutor trait,
//! enabling narratives to query Discord servers for real-time data.
//!
//! # Supported Commands
//!
//! - `server.get_stats` - Get server statistics (member count, channel count, etc.)
//! - `channels.list` - List all channels in a server
//! - `roles.list` - List all roles in a server
//!
//! # Example
//!
//! ```rust,ignore
//! use botticelli_social::DiscordCommandExecutor;
//! use std::collections::HashMap;
//!
//! // Create standalone executor
//! let executor = DiscordCommandExecutor::new("DISCORD_BOT_TOKEN");
//!
//! // Or create from existing bot
//! let bot = BotticelliBot::new(token, conn).await?;
//! let executor = DiscordCommandExecutor::with_http_client(bot.http_client());
//!
//! // Execute command
//! let mut args = HashMap::new();
//! args.insert("guild_id".to_string(), serde_json::json!("1234567890"));
//! let result = executor.execute("server.get_stats", &args).await?;
//! ```

use crate::{BotCommandError, BotCommandErrorKind, BotCommandExecutor, BotCommandResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use serenity::http::Http;
use serenity::model::id::GuildId;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Discord command executor for bot command execution.
///
/// Implements the BotCommandExecutor trait to provide Discord-specific
/// command handling using Serenity's HTTP client.
pub struct DiscordCommandExecutor {
    http: Arc<Http>,
}

impl DiscordCommandExecutor {
    /// Create a new Discord command executor with a bot token.
    ///
    /// This creates an independent HTTP client suitable for standalone use.
    /// The executor will make direct Discord API calls without requiring
    /// a running bot instance.
    ///
    /// # Arguments
    ///
    /// * `token` - Discord bot token from the Discord Developer Portal
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let executor = DiscordCommandExecutor::new("DISCORD_BOT_TOKEN");
    /// ```
    #[instrument(skip(token), fields(token_len = token.as_ref().len()))]
    pub fn new(token: impl AsRef<str>) -> Self {
        info!("Creating standalone Discord command executor");
        let http = Arc::new(Http::new(token.as_ref()));
        Self { http }
    }

    /// Create executor with an existing HTTP client.
    ///
    /// Use this to share the HTTP client with a running bot,
    /// coordinating rate limits and reducing connections.
    ///
    /// # Arguments
    ///
    /// * `http` - Arc reference to Serenity HTTP client
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bot = BotticelliBot::new(token, conn).await?;
    /// let executor = DiscordCommandExecutor::with_http_client(bot.http_client());
    /// ```
    pub fn with_http_client(http: Arc<Http>) -> Self {
        info!("Creating Discord command executor with shared HTTP client");
        Self { http }
    }

    /// Parse guild_id argument from command args.
    fn parse_guild_id(
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<GuildId> {
        let guild_id_value = args.get("guild_id").ok_or_else(|| {
            error!(command, "Missing required argument: guild_id");
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: command.to_string(),
                arg_name: "guild_id".to_string(),
            })
        })?;

        let guild_id_str = guild_id_value.as_str().ok_or_else(|| {
            error!(command, ?guild_id_value, "guild_id must be a string");
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: command.to_string(),
                arg_name: "guild_id".to_string(),
                reason: "Must be a string".to_string(),
            })
        })?;

        let guild_id_u64: u64 = guild_id_str.parse().map_err(|_| {
            error!(command, guild_id_str, "Invalid guild_id format");
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: command.to_string(),
                arg_name: "guild_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        Ok(GuildId::new(guild_id_u64))
    }

    /// Execute: server.get_stats
    ///
    /// Get server statistics including member count, channel count, role count, etc.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "server.get_stats",
            guild_id,
            member_count,
            channel_count
        )
    )]
    async fn server_get_stats(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("server.get_stats", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching guild stats from Discord API");

        // Fetch guild data
        let guild = self
            .http
            .get_guild(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch guild");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "server.get_stats".to_string(),
                    reason: format!("Failed to fetch guild: {}", e),
                })
            })?;

        // Fetch member count (guild.approximate_member_count is only available with partial guilds)
        // For now, we'll use the guild data we have
        let member_count = guild.approximate_member_count.unwrap_or(0);
        let channel_count = 0; // Would need separate API call to get channels

        tracing::Span::current().record("member_count", member_count);
        tracing::Span::current().record("channel_count", channel_count);

        let stats = serde_json::json!({
            "guild_id": guild.id.to_string(),
            "name": guild.name,
            "member_count": member_count,
            "description": guild.description,
            "icon_url": guild.icon_url(),
            "banner_url": guild.banner_url(),
            "owner_id": guild.owner_id.to_string(),
            "verification_level": format!("{:?}", guild.verification_level),
            "premium_tier": format!("{:?}", guild.premium_tier),
            "premium_subscription_count": guild.premium_subscription_count.unwrap_or(0),
        });

        info!(
            member_count,
            "Successfully retrieved guild stats"
        );

        Ok(stats)
    }

    /// Execute: channels.list
    ///
    /// List all channels in a server.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "channels.list",
            guild_id,
            channel_count
        )
    )]
    async fn channels_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("channels.list", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching channels from Discord API");

        // Fetch channels
        let channels = self
            .http
            .get_channels(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch channels");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "channels.list".to_string(),
                    reason: format!("Failed to fetch channels: {}", e),
                })
            })?;

        let channel_count = channels.len();
        tracing::Span::current().record("channel_count", channel_count);

        let channels_json: Vec<JsonValue> = channels
            .into_iter()
            .map(|channel| {
                serde_json::json!({
                    "id": channel.id.to_string(),
                    "name": channel.name,
                    "type": format!("{:?}", channel.kind),
                    "position": channel.position,
                    "topic": channel.topic,
                    "nsfw": channel.nsfw,
                    "parent_id": channel.parent_id.map(|id| id.to_string()),
                })
            })
            .collect();

        info!(channel_count, "Successfully retrieved channels");

        Ok(serde_json::json!(channels_json))
    }

    /// Execute: channels.get
    ///
    /// Get specific channel details.
    ///
    /// Required args: guild_id, channel_id
    #[instrument(
        skip(self, args),
        fields(
            command = "channels.get",
            guild_id,
            channel_id
        )
    )]
    async fn channels_get(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing arguments");
        let guild_id = Self::parse_guild_id("channels.get", args)?;

        // Parse channel_id
        let channel_id_value = args.get("channel_id").ok_or_else(|| {
            error!(command = "channels.get", "Missing required argument: channel_id");
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "channels.get".to_string(),
                arg_name: "channel_id".to_string(),
            })
        })?;

        let channel_id_str = channel_id_value.as_str().ok_or_else(|| {
            error!(command = "channels.get", ?channel_id_value, "channel_id must be a string");
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "channels.get".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Must be a string".to_string(),
            })
        })?;

        let channel_id_u64: u64 = channel_id_str.parse().map_err(|_| {
            error!(command = "channels.get", channel_id_str, "Invalid channel_id format");
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "channels.get".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        let channel_id = serenity::model::id::ChannelId::new(channel_id_u64);

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("channel_id", channel_id.get());
        info!(guild_id = %guild_id, channel_id = %channel_id, "Fetching channel from Discord API");

        // Fetch all channels and find the specific one
        let channels = self
            .http
            .get_channels(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch channels");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "channels.get".to_string(),
                    reason: format!("Failed to fetch channels: {}", e),
                })
            })?;

        // Find the specific channel
        let channel = channels
            .into_iter()
            .find(|c| c.id == channel_id)
            .ok_or_else(|| {
                error!(guild_id = %guild_id, channel_id = %channel_id, "Channel not found in guild");
                BotCommandError::new(BotCommandErrorKind::ResourceNotFound {
                    command: "channels.get".to_string(),
                    resource_type: "channel".to_string(),
                })
            })?;

        let channel_json = serde_json::json!({
            "id": channel.id.to_string(),
            "name": channel.name,
            "type": format!("{:?}", channel.kind),
            "position": channel.position,
            "topic": channel.topic,
            "nsfw": channel.nsfw,
            "parent_id": channel.parent_id.map(|id| id.to_string()),
            "rate_limit_per_user": channel.rate_limit_per_user,
            "bitrate": channel.bitrate,
        });

        info!(channel_id = %channel_id, "Successfully retrieved channel details");

        Ok(channel_json)
    }

    /// Execute: members.list
    ///
    /// List guild members (paginated).
    ///
    /// Required args: guild_id
    /// Optional args: limit (default 100, max 1000)
    #[instrument(
        skip(self, args),
        fields(
            command = "members.list",
            guild_id,
            limit,
            member_count
        )
    )]
    async fn members_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("members.list", args)?;

        // Parse optional limit parameter
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(100)
            .min(1000); // Discord's max is 1000

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("limit", limit);
        info!(guild_id = %guild_id, limit, "Fetching guild members from Discord API");

        // Fetch members
        let members = self
            .http
            .get_guild_members(guild_id, Some(limit), None)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch members");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "members.list".to_string(),
                    reason: format!("Failed to fetch members: {}", e),
                })
            })?;

        let member_count = members.len();
        tracing::Span::current().record("member_count", member_count);

        let members_json: Vec<JsonValue> = members
            .into_iter()
            .map(|member| {
                let roles: Vec<String> = member
                    .roles
                    .iter()
                    .map(|role_id| role_id.to_string())
                    .collect();

                serde_json::json!({
                    "user_id": member.user.id.to_string(),
                    "username": member.user.name,
                    "discriminator": member.user.discriminator,
                    "nickname": member.nick,
                    "roles": roles,
                    "joined_at": member.joined_at.map(|t| t.to_string()),
                    "premium_since": member.premium_since.map(|t| t.to_string()),
                    "avatar": member.avatar,
                    "pending": member.pending,
                    "deaf": member.deaf,
                    "mute": member.mute,
                })
            })
            .collect();

        info!(member_count, "Successfully retrieved guild members");

        Ok(serde_json::json!(members_json))
    }

    /// Execute: members.get
    ///
    /// Get specific member details.
    ///
    /// Required args: guild_id, user_id
    #[instrument(
        skip(self, args),
        fields(
            command = "members.get",
            guild_id,
            user_id
        )
    )]
    async fn members_get(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing arguments");
        let guild_id = Self::parse_guild_id("members.get", args)?;

        // Parse user_id
        let user_id_value = args.get("user_id").ok_or_else(|| {
            error!(command = "members.get", "Missing required argument: user_id");
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "members.get".to_string(),
                arg_name: "user_id".to_string(),
            })
        })?;

        let user_id_str = user_id_value.as_str().ok_or_else(|| {
            error!(command = "members.get", ?user_id_value, "user_id must be a string");
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "members.get".to_string(),
                arg_name: "user_id".to_string(),
                reason: "Must be a string".to_string(),
            })
        })?;

        let user_id_u64: u64 = user_id_str.parse().map_err(|_| {
            error!(command = "members.get", user_id_str, "Invalid user_id format");
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "members.get".to_string(),
                arg_name: "user_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        let user_id = serenity::model::id::UserId::new(user_id_u64);

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("user_id", user_id.get());
        info!(guild_id = %guild_id, user_id = %user_id, "Fetching member from Discord API");

        // Fetch member
        let member = self
            .http
            .get_member(guild_id, user_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, user_id = %user_id, error = %e, "Failed to fetch member");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "members.get".to_string(),
                    reason: format!("Failed to fetch member: {}", e),
                })
            })?;

        let roles: Vec<String> = member
            .roles
            .iter()
            .map(|role_id| role_id.to_string())
            .collect();

        let member_json = serde_json::json!({
            "user_id": member.user.id.to_string(),
            "username": member.user.name,
            "discriminator": member.user.discriminator,
            "nickname": member.nick,
            "roles": roles,
            "joined_at": member.joined_at.map(|t| t.to_string()),
            "premium_since": member.premium_since.map(|t| t.to_string()),
            "avatar": member.avatar,
            "pending": member.pending,
            "deaf": member.deaf,
            "mute": member.mute,
            "communication_disabled_until": member.communication_disabled_until.map(|t| t.to_string()),
        });

        info!(user_id = %user_id, "Successfully retrieved member details");

        Ok(member_json)
    }

    /// Execute: roles.list
    ///
    /// List all roles in a server.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "roles.list",
            guild_id,
            role_count
        )
    )]
    async fn roles_list(&self, args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("roles.list", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching roles from Discord API");

        // Fetch roles
        let roles = self
            .http
            .get_guild_roles(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch roles");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "roles.list".to_string(),
                    reason: format!("Failed to fetch roles: {}", e),
                })
            })?;

        let role_count = roles.len();
        tracing::Span::current().record("role_count", role_count);

        let roles_json: Vec<JsonValue> = roles
            .into_iter()
            .map(|role| {
                serde_json::json!({
                    "id": role.id.to_string(),
                    "name": role.name,
                    "color": role.colour.0,
                    "hoist": role.hoist,
                    "position": role.position,
                    "permissions": role.permissions.bits(),
                    "managed": role.managed,
                    "mentionable": role.mentionable,
                })
            })
            .collect();

        info!(role_count, "Successfully retrieved roles");

        Ok(serde_json::json!(roles_json))
    }

    /// Execute: roles.get
    ///
    /// Get specific role details.
    ///
    /// Required args: guild_id, role_id
    #[instrument(
        skip(self, args),
        fields(
            command = "roles.get",
            guild_id,
            role_id
        )
    )]
    async fn roles_get(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing arguments");
        let guild_id = Self::parse_guild_id("roles.get", args)?;

        // Parse role_id
        let role_id_value = args.get("role_id").ok_or_else(|| {
            error!(command = "roles.get", "Missing required argument: role_id");
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "roles.get".to_string(),
                arg_name: "role_id".to_string(),
            })
        })?;

        let role_id_str = role_id_value.as_str().ok_or_else(|| {
            error!(command = "roles.get", ?role_id_value, "role_id must be a string");
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "roles.get".to_string(),
                arg_name: "role_id".to_string(),
                reason: "Must be a string".to_string(),
            })
        })?;

        let role_id_u64: u64 = role_id_str.parse().map_err(|_| {
            error!(command = "roles.get", role_id_str, "Invalid role_id format");
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "roles.get".to_string(),
                arg_name: "role_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        let role_id = serenity::model::id::RoleId::new(role_id_u64);

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("role_id", role_id.get());
        info!(guild_id = %guild_id, role_id = %role_id, "Fetching role from Discord API");

        // Fetch all roles and find the specific one
        let roles = self
            .http
            .get_guild_roles(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch roles");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "roles.get".to_string(),
                    reason: format!("Failed to fetch roles: {}", e),
                })
            })?;

        // Find the specific role
        let role = roles
            .into_iter()
            .find(|r| r.id == role_id)
            .ok_or_else(|| {
                error!(guild_id = %guild_id, role_id = %role_id, "Role not found in guild");
                BotCommandError::new(BotCommandErrorKind::ResourceNotFound {
                    command: "roles.get".to_string(),
                    resource_type: "role".to_string(),
                })
            })?;

        let role_json = serde_json::json!({
            "id": role.id.to_string(),
            "name": role.name,
            "color": role.colour.0,
            "hoist": role.hoist,
            "position": role.position,
            "permissions": role.permissions.bits(),
            "managed": role.managed,
            "mentionable": role.mentionable,
            "icon": role.icon,
            "unicode_emoji": role.unicode_emoji,
        });

        info!(role_id = %role_id, "Successfully retrieved role details");

        Ok(role_json)
    }

    /// Execute: emojis.list
    ///
    /// List custom emojis in a server.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "emojis.list",
            guild_id,
            emoji_count
        )
    )]
    async fn emojis_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("emojis.list", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching emojis from Discord API");

        // Fetch emojis
        let emojis = self
            .http
            .get_emojis(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch emojis");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "emojis.list".to_string(),
                    reason: format!("Failed to fetch emojis: {}", e),
                })
            })?;

        let emoji_count = emojis.len();
        tracing::Span::current().record("emoji_count", emoji_count);

        let emojis_json: Vec<JsonValue> = emojis
            .into_iter()
            .map(|emoji| {
                serde_json::json!({
                    "id": emoji.id.to_string(),
                    "name": emoji.name,
                    "animated": emoji.animated,
                    "managed": emoji.managed,
                    "require_colons": emoji.require_colons,
                    "available": emoji.available,
                })
            })
            .collect();

        info!(emoji_count, "Successfully retrieved emojis");

        Ok(serde_json::json!(emojis_json))
    }

    /// Execute: events.list
    ///
    /// List scheduled events in a server.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "events.list",
            guild_id,
            event_count
        )
    )]
    async fn events_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("events.list", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching scheduled events from Discord API");

        // Fetch scheduled events
        let events = self
            .http
            .get_scheduled_events(guild_id, false)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch events");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "events.list".to_string(),
                    reason: format!("Failed to fetch events: {}", e),
                })
            })?;

        let event_count = events.len();
        tracing::Span::current().record("event_count", event_count);

        let events_json: Vec<JsonValue> = events
            .into_iter()
            .map(|event| {
                serde_json::json!({
                    "id": event.id.to_string(),
                    "name": event.name,
                    "description": event.description,
                    "start_time": event.start_time.to_string(),
                    "end_time": event.end_time.map(|t| t.to_string()),
                    "status": format!("{:?}", event.status),
                    "kind": format!("{:?}", event.kind),
                    "user_count": event.user_count,
                })
            })
            .collect();

        info!(event_count, "Successfully retrieved events");

        Ok(serde_json::json!(events_json))
    }

    /// Execute: stickers.list
    ///
    /// List custom stickers in a server.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "stickers.list",
            guild_id,
            sticker_count
        )
    )]
    async fn stickers_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("stickers.list", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching stickers from Discord API");

        // Fetch stickers
        let stickers = self
            .http
            .get_guild_stickers(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch stickers");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "stickers.list".to_string(),
                    reason: format!("Failed to fetch stickers: {}", e),
                })
            })?;

        let sticker_count = stickers.len();
        tracing::Span::current().record("sticker_count", sticker_count);

        let stickers_json: Vec<JsonValue> = stickers
            .into_iter()
            .map(|sticker| {
                serde_json::json!({
                    "id": sticker.id.to_string(),
                    "name": sticker.name,
                    "description": sticker.description,
                    "tags": sticker.tags,
                    "format_type": format!("{:?}", sticker.format_type),
                    "available": sticker.available,
                })
            })
            .collect();

        info!(sticker_count, "Successfully retrieved stickers");

        Ok(serde_json::json!(stickers_json))
    }

    /// Execute: invites.list
    ///
    /// List active invites in a server.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "invites.list",
            guild_id,
            invite_count
        )
    )]
    async fn invites_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("invites.list", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching invites from Discord API");

        // Fetch invites
        let invites = self
            .http
            .get_guild_invites(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch invites");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "invites.list".to_string(),
                    reason: format!("Failed to fetch invites: {}", e),
                })
            })?;

        let invite_count = invites.len();
        tracing::Span::current().record("invite_count", invite_count);

        let invites_json: Vec<JsonValue> = invites
            .into_iter()
            .map(|invite| {
                serde_json::json!({
                    "code": invite.code,
                    "channel_id": invite.channel.id.to_string(),
                    "inviter": invite.inviter.as_ref().map(|u| serde_json::json!({
                        "id": u.id.to_string(),
                        "name": u.name.clone(),
                    })),
                    "uses": invite.uses,
                    "max_uses": invite.max_uses,
                    "max_age": invite.max_age,
                    "temporary": invite.temporary,
                    "created_at": invite.created_at.to_string(),
                })
            })
            .collect();

        info!(invite_count, "Successfully retrieved invites");

        Ok(serde_json::json!(invites_json))
    }

    /// Execute: webhooks.list
    ///
    /// List webhooks in a server.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "webhooks.list",
            guild_id,
            webhook_count
        )
    )]
    async fn webhooks_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("webhooks.list", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching webhooks from Discord API");

        // Fetch webhooks
        let webhooks = self
            .http
            .get_guild_webhooks(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch webhooks");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "webhooks.list".to_string(),
                    reason: format!("Failed to fetch webhooks: {}", e),
                })
            })?;

        let webhook_count = webhooks.len();
        tracing::Span::current().record("webhook_count", webhook_count);

        let webhooks_json: Vec<JsonValue> = webhooks
            .into_iter()
            .map(|webhook| {
                serde_json::json!({
                    "id": webhook.id.to_string(),
                    "name": webhook.name,
                    "channel_id": webhook.channel_id.map(|id| id.to_string()),
                    "avatar": webhook.avatar,
                    "guild_id": webhook.guild_id.map(|id| id.to_string()),
                })
            })
            .collect();

        info!(webhook_count, "Successfully retrieved webhooks");

        Ok(serde_json::json!(webhooks_json))
    }

    /// Execute: bans.list
    ///
    /// List banned users in a server.
    ///
    /// Required args: guild_id
    /// Optional args: limit (default 100, max 1000)
    #[instrument(
        skip(self, args),
        fields(
            command = "bans.list",
            guild_id,
            limit,
            ban_count
        )
    )]
    async fn bans_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("bans.list", args)?;

        // Parse optional limit parameter
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| l.min(1000) as u8);

        tracing::Span::current().record("guild_id", guild_id.get());
        if let Some(limit) = limit {
            tracing::Span::current().record("limit", limit);
        }
        info!(guild_id = %guild_id, ?limit, "Fetching bans from Discord API");

        // Fetch bans
        let bans = self
            .http
            .get_bans(guild_id, None, limit)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch bans");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "bans.list".to_string(),
                    reason: format!("Failed to fetch bans: {}", e),
                })
            })?;

        let ban_count = bans.len();
        tracing::Span::current().record("ban_count", ban_count);

        let bans_json: Vec<JsonValue> = bans
            .into_iter()
            .map(|ban| {
                serde_json::json!({
                    "user_id": ban.user.id.to_string(),
                    "username": ban.user.name,
                    "reason": ban.reason,
                })
            })
            .collect();

        info!(ban_count, "Successfully retrieved bans");

        Ok(serde_json::json!(bans_json))
    }

    /// Execute: integrations.list
    ///
    /// List integrations in a server.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "integrations.list",
            guild_id,
            integration_count
        )
    )]
    async fn integrations_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("integrations.list", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching integrations from Discord API");

        // Fetch integrations
        let integrations = self
            .http
            .get_guild_integrations(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch integrations");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "integrations.list".to_string(),
                    reason: format!("Failed to fetch integrations: {}", e),
                })
            })?;

        let integration_count = integrations.len();
        tracing::Span::current().record("integration_count", integration_count);

        let integrations_json: Vec<JsonValue> = integrations
            .into_iter()
            .map(|integration| {
                serde_json::json!({
                    "id": integration.id.to_string(),
                    "name": integration.name,
                    "type": integration.kind,
                    "enabled": integration.enabled,
                    "syncing": integration.syncing,
                    "account": serde_json::json!({
                        "id": integration.account.id,
                        "name": integration.account.name,
                    }),
                })
            })
            .collect();

        info!(integration_count, "Successfully retrieved integrations");

        Ok(serde_json::json!(integrations_json))
    }

    /// Execute: voice_regions.list
    ///
    /// List available voice regions for a server.
    ///
    /// Required args: guild_id
    #[instrument(
        skip(self, args),
        fields(
            command = "voice_regions.list",
            guild_id,
            region_count
        )
    )]
    async fn voice_regions_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        debug!("Parsing guild_id argument");
        let guild_id = Self::parse_guild_id("voice_regions.list", args)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        info!(guild_id = %guild_id, "Fetching voice regions from Discord API");

        // Fetch voice regions
        let regions = self
            .http
            .get_guild_regions(guild_id)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to fetch voice regions");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "voice_regions.list".to_string(),
                    reason: format!("Failed to fetch voice regions: {}", e),
                })
            })?;

        let region_count = regions.len();
        tracing::Span::current().record("region_count", region_count);

        let regions_json: Vec<JsonValue> = regions
            .into_iter()
            .map(|region| {
                serde_json::json!({
                    "id": region.id,
                    "name": region.name,
                    "optimal": region.optimal,
                    "deprecated": region.deprecated,
                    "custom": region.custom,
                })
            })
            .collect();

        info!(region_count, "Successfully retrieved voice regions");

        Ok(serde_json::json!(regions_json))
    }

    // =============================================================================
    // WRITE COMMANDS (Require Security Framework)
    // =============================================================================

    /// Send a message to a channel.
    ///
    /// **Security**: This command MUST go through the security framework.
    /// Use `SecureBotCommandExecutor` to ensure proper permission checking,
    /// content validation, rate limiting, and approval workflows.
    ///
    /// # Required Arguments
    ///
    /// * `guild_id` - Guild ID
    /// * `channel_id` - Channel ID
    /// * `content` - Message content (max 2000 characters)
    ///
    /// # Optional Arguments
    ///
    /// * `tts` - Enable text-to-speech (default: false)
    ///
    /// # Returns
    ///
    /// ```json
    /// {
    ///     "id": "message_id",
    ///     "channel_id": "channel_id",
    ///     "content": "message_content",
    ///     "timestamp": "2024-01-01T00:00:00Z"
    /// }
    /// ```
    #[instrument(
        skip(self, args),
        fields(
            command = "messages.send",
            guild_id,
            channel_id,
            content_len
        )
    )]
    async fn messages_send(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::model::id::ChannelId;
        use serenity::builder::CreateMessage;

        debug!("Parsing arguments for messages.send");
        let _guild_id = Self::parse_guild_id("messages.send", args)?;
        
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "messages.send".to_string(),
                    arg_name: "channel_id".to_string(),
                })
            })?;
        
        let channel_id = channel_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.send".to_string(),
                arg_name: "channel_id".to_string(),
                reason: format!("Invalid channel ID format: {}", e),
            })
        })?;
        let channel_id = ChannelId::new(channel_id);

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "messages.send".to_string(),
                    arg_name: "content".to_string(),
                })
            })?
            .to_string();

        let tts = args
            .get("tts")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        tracing::Span::current().record("channel_id", channel_id.get());
        tracing::Span::current().record("content_len", content.len());

        info!(
            channel_id = %channel_id,
            content_len = content.len(),
            tts,
            "Sending message to Discord channel"
        );

        // Send the message
        let message = channel_id
            .send_message(&self.http, CreateMessage::new().content(content).tts(tts))
            .await
            .map_err(|e| {
                error!(channel_id = %channel_id, error = %e, "Failed to send message");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "messages.send".to_string(),
                    reason: format!("Failed to send message: {}", e),
                })
            })?;

        info!(message_id = %message.id, "Successfully sent message");

        Ok(serde_json::json!({
            "id": message.id.to_string(),
            "channel_id": message.channel_id.to_string(),
            "content": message.content,
            "timestamp": message.timestamp.to_rfc3339(),
            "tts": message.tts,
        }))
    }

    /// Create a new channel in a guild.
    ///
    /// **Security**: This command MUST go through the security framework
    /// and typically requires approval workflow.
    ///
    /// # Required Arguments
    ///
    /// * `guild_id` - Guild ID
    /// * `name` - Channel name (2-100 characters)
    /// * `kind` - Channel type ("text", "voice", "category", "announcement", "stage", "forum")
    ///
    /// # Optional Arguments
    ///
    /// * `topic` - Channel topic (max 1024 characters for text channels)
    /// * `position` - Sorting position
    /// * `nsfw` - Age-restricted channel (default: false)
    /// * `category_id` - Parent category ID
    ///
    /// # Returns
    ///
    /// ```json
    /// {
    ///     "id": "channel_id",
    ///     "name": "channel-name",
    ///     "kind": "text",
    ///     "position": 0
    /// }
    /// ```
    #[instrument(
        skip(self, args),
        fields(
            command = "channels.create",
            guild_id,
            name,
            kind
        )
    )]
    async fn channels_create(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::builder::CreateChannel;
        use serenity::model::channel::ChannelType;

        debug!("Parsing arguments for channels.create");
        let guild_id = Self::parse_guild_id("channels.create", args)?;

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "channels.create".to_string(),
                    arg_name: "name".to_string(),
                })
            })?;

        let kind_str = args
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "channels.create".to_string(),
                    arg_name: "kind".to_string(),
                })
            })?;

        let kind = match kind_str {
            "text" => ChannelType::Text,
            "voice" => ChannelType::Voice,
            "category" => ChannelType::Category,
            "announcement" => ChannelType::News,
            "stage" => ChannelType::Stage,
            "forum" => ChannelType::Forum,
            _ => {
                return Err(BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "channels.create".to_string(),
                    arg_name: "kind".to_string(),
                    reason: format!("Invalid channel type: {}", kind_str),
                }));
            }
        };

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("name", name);
        tracing::Span::current().record("kind", kind_str);

        info!(
            guild_id = %guild_id,
            name,
            kind = kind_str,
            "Creating channel in Discord guild"
        );

        // Build the create channel request
        let mut builder = CreateChannel::new(name).kind(kind);

        if let Some(topic) = args.get("topic").and_then(|v| v.as_str()) {
            builder = builder.topic(topic);
        }

        if let Some(position) = args.get("position").and_then(|v| v.as_u64()) {
            builder = builder.position(position as u16);
        }

        if let Some(nsfw) = args.get("nsfw").and_then(|v| v.as_bool()) {
            builder = builder.nsfw(nsfw);
        }

        // Create the channel
        let channel = guild_id
            .create_channel(&self.http, builder)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, name, error = %e, "Failed to create channel");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "channels.create".to_string(),
                    reason: format!("Failed to create channel: {}", e),
                })
            })?;

        info!(channel_id = %channel.id, name, "Successfully created channel");

        Ok(serde_json::json!({
            "id": channel.id.to_string(),
            "name": channel.name,
            "kind": format!("{:?}", channel.kind),
            "position": channel.position,
        }))
    }

    /// Delete a channel from a guild.
    ///
    /// **Security**: This command MUST go through the security framework
    /// and ALWAYS requires approval workflow.
    ///
    /// # Required Arguments
    ///
    /// * `guild_id` - Guild ID
    /// * `channel_id` - Channel ID to delete
    ///
    /// # Optional Arguments
    ///
    /// * `reason` - Audit log reason (max 512 characters)
    ///
    /// # Returns
    ///
    /// ```json
    /// {
    ///     "id": "channel_id",
    ///     "deleted": true
    /// }
    /// ```
    #[instrument(
        skip(self, args),
        fields(
            command = "channels.delete",
            guild_id,
            channel_id
        )
    )]
    async fn channels_delete(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::model::id::ChannelId;

        debug!("Parsing arguments for channels.delete");
        let guild_id = Self::parse_guild_id("channels.delete", args)?;

        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "channels.delete".to_string(),
                    arg_name: "channel_id".to_string(),
                })
            })?;

        let channel_id = channel_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "channels.delete".to_string(),
                arg_name: "channel_id".to_string(),
                reason: format!("Invalid channel ID format: {}", e),
            })
        })?;
        let channel_id = ChannelId::new(channel_id);

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("channel_id", channel_id.get());

        warn!(
            guild_id = %guild_id,
            channel_id = %channel_id,
            "Deleting channel from Discord guild"
        );

        // Delete the channel
        channel_id
            .delete(&self.http)
            .await
            .map_err(|e| {
                error!(
                    guild_id = %guild_id,
                    channel_id = %channel_id,
                    error = %e,
                    "Failed to delete channel"
                );
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "channels.delete".to_string(),
                    reason: format!("Failed to delete channel: {}", e),
                })
            })?;

        info!(channel_id = %channel_id, "Successfully deleted channel");

        Ok(serde_json::json!({
            "id": channel_id.to_string(),
            "deleted": true,
        }))
    }

    /// Ban a member from a guild.
    ///
    /// **Security**: This command MUST go through the security framework
    /// and ALWAYS requires approval workflow.
    ///
    /// # Required Arguments
    ///
    /// * `guild_id` - Guild ID
    /// * `user_id` - User ID to ban
    ///
    /// # Optional Arguments
    ///
    /// * `delete_message_days` - Delete messages from last N days (0-7, default: 0)
    /// * `reason` - Ban reason for audit log (max 512 characters)
    ///
    /// # Returns
    ///
    /// ```json
    /// {
    ///     "user_id": "user_id",
    ///     "banned": true
    /// }
    /// ```
    #[instrument(
        skip(self, args),
        fields(
            command = "members.ban",
            guild_id,
            user_id
        )
    )]
    async fn members_ban(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::model::id::UserId;

        debug!("Parsing arguments for members.ban");
        let guild_id = Self::parse_guild_id("members.ban", args)?;

        let user_id_str = args
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "members.ban".to_string(),
                    arg_name: "user_id".to_string(),
                })
            })?;

        let user_id = user_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "members.ban".to_string(),
                arg_name: "user_id".to_string(),
                reason: format!("Invalid user ID format: {}", e),
            })
        })?;
        let user_id = UserId::new(user_id);

        let delete_message_days = args
            .get("delete_message_days")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .min(7) as u8;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("user_id", user_id.get());

        warn!(
            guild_id = %guild_id,
            user_id = %user_id,
            delete_message_days,
            "Banning member from Discord guild"
        );

        // Ban the member
        guild_id
            .ban(&self.http, user_id, delete_message_days)
            .await
            .map_err(|e| {
                error!(
                    guild_id = %guild_id,
                    user_id = %user_id,
                    error = %e,
                    "Failed to ban member"
                );
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "members.ban".to_string(),
                    reason: format!("Failed to ban member: {}", e),
                })
            })?;

        info!(user_id = %user_id, "Successfully banned member");

        Ok(serde_json::json!({
            "user_id": user_id.to_string(),
            "banned": true,
        }))
    }
}

#[async_trait]
impl BotCommandExecutor for DiscordCommandExecutor {
    fn platform(&self) -> &str {
        "discord"
    }

    #[instrument(
        skip(self, args),
        fields(
            platform = "discord",
            command,
            arg_count = args.len(),
            result_size,
            duration_ms
        )
    )]
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        info!("Executing Discord bot command");

        let start = std::time::Instant::now();

        let result = match command {
            // Read commands
            "server.get_stats" => self.server_get_stats(args).await?,
            "channels.list" => self.channels_list(args).await?,
            "channels.get" => self.channels_get(args).await?,
            "roles.list" => self.roles_list(args).await?,
            "roles.get" => self.roles_get(args).await?,
            "members.list" => self.members_list(args).await?,
            "members.get" => self.members_get(args).await?,
            "emojis.list" => self.emojis_list(args).await?,
            "events.list" => self.events_list(args).await?,
            "stickers.list" => self.stickers_list(args).await?,
            "invites.list" => self.invites_list(args).await?,
            "webhooks.list" => self.webhooks_list(args).await?,
            "bans.list" => self.bans_list(args).await?,
            "integrations.list" => self.integrations_list(args).await?,
            "voice_regions.list" => self.voice_regions_list(args).await?,
            // Write commands (require security framework)
            "messages.send" => self.messages_send(args).await?,
            "channels.create" => self.channels_create(args).await?,
            "channels.delete" => self.channels_delete(args).await?,
            "members.ban" => self.members_ban(args).await?,
            _ => {
                error!(
                    command,
                    supported = ?self.supported_commands(),
                    "Command not found"
                );
                return Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
                    command.to_string(),
                )));
            }
        };

        let duration_ms = start.elapsed().as_millis();
        let result_size = serde_json::to_string(&result)
            .map(|s| s.len())
            .unwrap_or(0);

        tracing::Span::current().record("duration_ms", duration_ms);
        tracing::Span::current().record("result_size", result_size);
        info!(
            duration_ms,
            result_size,
            "Discord command executed successfully"
        );

        Ok(result)
    }

    fn supports_command(&self, command: &str) -> bool {
        matches!(
            command,
            // Read commands
            "server.get_stats"
                | "channels.list"
                | "channels.get"
                | "roles.list"
                | "roles.get"
                | "members.list"
                | "members.get"
                | "emojis.list"
                | "events.list"
                | "stickers.list"
                | "invites.list"
                | "webhooks.list"
                | "bans.list"
                | "integrations.list"
                | "voice_regions.list"
                // Write commands
                | "messages.send"
                | "channels.create"
                | "channels.delete"
                | "members.ban"
        )
    }

    fn supported_commands(&self) -> Vec<String> {
        vec![
            // Read commands
            "server.get_stats".to_string(),
            "channels.list".to_string(),
            "channels.get".to_string(),
            "roles.list".to_string(),
            "roles.get".to_string(),
            "members.list".to_string(),
            "members.get".to_string(),
            "emojis.list".to_string(),
            "events.list".to_string(),
            "stickers.list".to_string(),
            "invites.list".to_string(),
            "webhooks.list".to_string(),
            "bans.list".to_string(),
            "integrations.list".to_string(),
            "voice_regions.list".to_string(),
            // Write commands
            "messages.send".to_string(),
            "channels.create".to_string(),
            "channels.delete".to_string(),
            "members.ban".to_string(),
        ]
    }

    fn command_help(&self, command: &str) -> Option<String> {
        match command {
            "server.get_stats" => Some(
                "Get server statistics (member count, channels, etc.)\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "channels.list" => Some(
                "List all channels in a server\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "channels.get" => Some(
                "Get specific channel details\n\
                 Required arguments: guild_id, channel_id"
                    .to_string(),
            ),
            "roles.list" => Some(
                "List all roles in a server\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "roles.get" => Some(
                "Get specific role details\n\
                 Required arguments: guild_id, role_id"
                    .to_string(),
            ),
            "members.list" => Some(
                "List guild members (paginated)\n\
                 Required arguments: guild_id\n\
                 Optional arguments: limit (default 100, max 1000)"
                    .to_string(),
            ),
            "members.get" => Some(
                "Get specific member details\n\
                 Required arguments: guild_id, user_id"
                    .to_string(),
            ),
            "emojis.list" => Some(
                "List custom emojis in a server\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "events.list" => Some(
                "List scheduled events in a server\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "stickers.list" => Some(
                "List custom stickers in a server\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "invites.list" => Some(
                "List active invites in a server\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "webhooks.list" => Some(
                "List webhooks in a server\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "bans.list" => Some(
                "List banned users in a server\n\
                 Required arguments: guild_id\n\
                 Optional arguments: limit (default 100, max 1000)"
                    .to_string(),
            ),
            "integrations.list" => Some(
                "List integrations in a server\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "voice_regions.list" => Some(
                "List available voice regions for a server\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "messages.send" => Some(
                "Send a message to a channel (requires security framework)\n\
                 Required arguments: guild_id, channel_id, content\n\
                 Optional arguments: tts (default false)"
                    .to_string(),
            ),
            "channels.create" => Some(
                "Create a new channel (requires security framework and approval)\n\
                 Required arguments: guild_id, name, kind (text/voice/category/announcement/stage/forum)\n\
                 Optional arguments: topic, position, nsfw, category_id"
                    .to_string(),
            ),
            "channels.delete" => Some(
                "Delete a channel (requires security framework and approval)\n\
                 Required arguments: guild_id, channel_id\n\
                 Optional arguments: reason"
                    .to_string(),
            ),
            "members.ban" => Some(
                "Ban a member (requires security framework and approval)\n\
                 Required arguments: guild_id, user_id\n\
                 Optional arguments: delete_message_days (0-7), reason"
                    .to_string(),
            ),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_command() {
        let token = "test_token";
        let executor = DiscordCommandExecutor::new(token);

        assert!(executor.supports_command("server.get_stats"));
        assert!(executor.supports_command("channels.list"));
        assert!(executor.supports_command("channels.get"));
        assert!(executor.supports_command("roles.list"));
        assert!(executor.supports_command("roles.get"));
        assert!(executor.supports_command("members.list"));
        assert!(executor.supports_command("members.get"));
        assert!(executor.supports_command("emojis.list"));
        assert!(executor.supports_command("events.list"));
        assert!(executor.supports_command("stickers.list"));
        assert!(executor.supports_command("invites.list"));
        assert!(executor.supports_command("webhooks.list"));
        assert!(executor.supports_command("bans.list"));
        assert!(executor.supports_command("integrations.list"));
        assert!(executor.supports_command("voice_regions.list"));
        assert!(!executor.supports_command("unknown.command"));
    }

    #[test]
    fn test_supported_commands() {
        let token = "test_token";
        let executor = DiscordCommandExecutor::new(token);

        let commands = executor.supported_commands();
        assert_eq!(commands.len(), 15);
        assert!(commands.contains(&"server.get_stats".to_string()));
        assert!(commands.contains(&"channels.list".to_string()));
        assert!(commands.contains(&"channels.get".to_string()));
        assert!(commands.contains(&"roles.list".to_string()));
        assert!(commands.contains(&"roles.get".to_string()));
        assert!(commands.contains(&"members.list".to_string()));
        assert!(commands.contains(&"members.get".to_string()));
        assert!(commands.contains(&"emojis.list".to_string()));
        assert!(commands.contains(&"events.list".to_string()));
        assert!(commands.contains(&"stickers.list".to_string()));
        assert!(commands.contains(&"invites.list".to_string()));
        assert!(commands.contains(&"webhooks.list".to_string()));
        assert!(commands.contains(&"bans.list".to_string()));
        assert!(commands.contains(&"integrations.list".to_string()));
        assert!(commands.contains(&"voice_regions.list".to_string()));
    }

    #[test]
    fn test_command_help() {
        let token = "test_token";
        let executor = DiscordCommandExecutor::new(token);

        assert!(executor.command_help("server.get_stats").is_some());
        assert!(executor.command_help("channels.list").is_some());
        assert!(executor.command_help("channels.get").is_some());
        assert!(executor.command_help("roles.list").is_some());
        assert!(executor.command_help("roles.get").is_some());
        assert!(executor.command_help("members.list").is_some());
        assert!(executor.command_help("members.get").is_some());
        assert!(executor.command_help("emojis.list").is_some());
        assert!(executor.command_help("events.list").is_some());
        assert!(executor.command_help("stickers.list").is_some());
        assert!(executor.command_help("invites.list").is_some());
        assert!(executor.command_help("webhooks.list").is_some());
        assert!(executor.command_help("bans.list").is_some());
        assert!(executor.command_help("integrations.list").is_some());
        assert!(executor.command_help("voice_regions.list").is_some());
        assert!(executor.command_help("unknown.command").is_none());
    }

    #[test]
    fn test_platform() {
        let token = "test_token";
        let executor = DiscordCommandExecutor::new(token);

        assert_eq!(executor.platform(), "discord");
    }
}
