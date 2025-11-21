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
use botticelli_security::PermissionChecker;
use derive_getters::Getters;
use derive_setters::Setters;
use serde_json::Value as JsonValue;
use serenity::http::Http;
use serenity::model::id::GuildId;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};

/// Discord command executor for bot command execution.
///
/// Implements the BotCommandExecutor trait to provide Discord-specific
/// command handling using Serenity's HTTP client.
#[derive(Getters, Setters)]
#[setters(prefix = "with_")]
pub struct DiscordCommandExecutor {
    /// Serenity HTTP client for Discord API calls
    http: Arc<Http>,
    /// Optional security policy checker for command authorization
    #[setters(skip)]  // Manual setter with custom logic
    permission_checker: Option<Arc<PermissionChecker>>,
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
        Self {
            http,
            permission_checker: None,
        }
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
        Self {
            http,
            permission_checker: None,
        }
    }

    /// Set permission checker for security enforcement.
    ///
    /// Write operations require a permission checker with appropriate policies.
    pub fn with_permission_checker(mut self, checker: Arc<PermissionChecker>) -> Self {
        info!("Setting permission checker for Discord command executor");
        self.permission_checker = Some(checker);
        self
    }

    /// Check permission for a write operation.
    /// TODO: Properly integrate with security framework
    #[allow(dead_code)]
    fn check_permission(&self, _command: &str, _resource_id: &str) -> BotCommandResult<()> {
        // Temporarily disabled until we properly integrate security framework
        Ok(())
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

    /// Execute: messages.get
    ///
    /// Get a specific message from a channel.
    ///
    /// Required args: channel_id, message_id
    #[instrument(
        skip(self, args),
        fields(
            command = "messages.get",
            channel_id,
            message_id
        )
    )]
    async fn messages_get(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        // Parse channel_id
        let channel_id_value = args.get("channel_id").ok_or_else(|| {
            error!(command = "messages.get", "Missing required argument: channel_id");
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "messages.get".to_string(),
                arg_name: "channel_id".to_string(),
            })
        })?;

        let channel_id_str = channel_id_value.as_str().ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.get".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Must be a string".to_string(),
            })
        })?;

        let channel_id: u64 = channel_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.get".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        // Parse message_id
        let message_id_value = args.get("message_id").ok_or_else(|| {
            error!(command = "messages.get", "Missing required argument: message_id");
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "messages.get".to_string(),
                arg_name: "message_id".to_string(),
            })
        })?;

        let message_id_str = message_id_value.as_str().ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.get".to_string(),
                arg_name: "message_id".to_string(),
                reason: "Must be a string".to_string(),
            })
        })?;

        let message_id: u64 = message_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.get".to_string(),
                arg_name: "message_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        tracing::Span::current().record("channel_id", channel_id);
        tracing::Span::current().record("message_id", message_id);
        info!(channel_id, message_id, "Fetching message from Discord API");

        let message = self
            .http
            .get_message(channel_id.into(), message_id.into())
            .await
            .map_err(|e| {
                error!(channel_id, message_id, error = %e, "Failed to fetch message");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "messages.get".to_string(),
                    reason: format!("Failed to fetch message: {}", e),
                })
            })?;

        let message_json = serde_json::json!({
            "id": message.id.to_string(),
            "content": message.content,
            "author": {
                "id": message.author.id.to_string(),
                "name": message.author.name,
                "discriminator": message.author.discriminator,
                "bot": message.author.bot,
            },
            "timestamp": message.timestamp.to_string(),
            "edited_timestamp": message.edited_timestamp.map(|t| t.to_string()),
            "tts": message.tts,
            "mention_everyone": message.mention_everyone,
            "mentions": message.mentions.iter().map(|u| u.id.to_string()).collect::<Vec<_>>(),
            "attachments": message.attachments.len(),
            "embeds": message.embeds.len(),
            "reactions": message.reactions.iter().map(|r| serde_json::json!({
                "emoji": r.reaction_type.to_string(),
                "count": r.count,
            })).collect::<Vec<_>>(),
            "pinned": message.pinned,
        });

        info!("Successfully retrieved message");
        Ok(message_json)
    }

    /// Execute: messages.list
    ///
    /// List messages from a channel (message history).
    ///
    /// Required args: channel_id
    /// Optional args: limit (default: 50, max: 100)
    #[instrument(
        skip(self, args),
        fields(
            command = "messages.list",
            channel_id,
            limit,
            message_count
        )
    )]
    async fn messages_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        // Parse channel_id
        let channel_id_value = args.get("channel_id").ok_or_else(|| {
            error!(command = "messages.list", "Missing required argument: channel_id");
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "messages.list".to_string(),
                arg_name: "channel_id".to_string(),
            })
        })?;

        let channel_id_str = channel_id_value.as_str().ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.list".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Must be a string".to_string(),
            })
        })?;

        let channel_id: u64 = channel_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.list".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        // Parse optional limit
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| l.min(100) as u8)
            .unwrap_or(50);

        tracing::Span::current().record("channel_id", channel_id);
        tracing::Span::current().record("limit", limit);
        info!(channel_id, limit, "Fetching messages from Discord API");

        let messages = self
            .http
            .get_messages(channel_id.into(), None, Some(limit))
            .await
            .map_err(|e| {
                error!(channel_id, error = %e, "Failed to fetch messages");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "messages.list".to_string(),
                    reason: format!("Failed to fetch messages: {}", e),
                })
            })?;

        let message_count = messages.len();
        tracing::Span::current().record("message_count", message_count);

        let messages_json: Vec<JsonValue> = messages
            .into_iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id.to_string(),
                    "content": m.content,
                    "author": {
                        "id": m.author.id.to_string(),
                        "name": m.author.name,
                        "bot": m.author.bot,
                    },
                    "timestamp": m.timestamp.to_string(),
                    "attachments": m.attachments.len(),
                    "embeds": m.embeds.len(),
                })
            })
            .collect();

        info!(message_count, "Successfully retrieved messages");
        Ok(serde_json::json!(messages_json))
    }

    /// Execute: messages.edit
    ///
    /// Edit an existing message.
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: channel_id, message_id, content
    #[instrument(
        skip(self, args),
        fields(
            command = "messages.edit",
            channel_id,
            message_id
        )
    )]
    async fn messages_edit(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        // Parse channel_id
        let channel_id_value = args.get("channel_id").ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "messages.edit".to_string(),
                arg_name: "channel_id".to_string(),
            })
        })?;

        let channel_id: u64 = channel_id_value
            .as_str()
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "messages.edit".to_string(),
                    arg_name: "channel_id".to_string(),
                    reason: "Must be a string".to_string(),
                })
            })?
            .parse()
            .map_err(|_| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "messages.edit".to_string(),
                    arg_name: "channel_id".to_string(),
                    reason: "Invalid Discord ID format".to_string(),
                })
            })?;

        // Parse message_id
        let message_id_value = args.get("message_id").ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "messages.edit".to_string(),
                arg_name: "message_id".to_string(),
            })
        })?;

        let message_id: u64 = message_id_value
            .as_str()
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "messages.edit".to_string(),
                    arg_name: "message_id".to_string(),
                    reason: "Must be a string".to_string(),
                })
            })?
            .parse()
            .map_err(|_| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "messages.edit".to_string(),
                    arg_name: "message_id".to_string(),
                    reason: "Invalid Discord ID format".to_string(),
                })
            })?;

        // Parse content
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "messages.edit".to_string(),
                    arg_name: "content".to_string(),
                })
            })?;

        tracing::Span::current().record("channel_id", channel_id);
        tracing::Span::current().record("message_id", message_id);
        info!(channel_id, message_id, "Editing message via Discord API");

        use serenity::builder::EditMessage;
        let builder = EditMessage::new().content(content);

        let edited_message = self
            .http
            .edit_message(channel_id.into(), message_id.into(), &builder, Vec::new())
            .await
            .map_err(|e| {
                error!(channel_id, message_id, error = %e, "Failed to edit message");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "messages.edit".to_string(),
                    reason: format!("Failed to edit message: {}", e),
                })
            })?;

        info!("Successfully edited message");
        Ok(serde_json::json!({
            "id": edited_message.id.to_string(),
            "content": edited_message.content,
            "edited_timestamp": edited_message.edited_timestamp.map(|t| t.to_string()),
        }))
    }

    /// Execute: messages.delete
    ///
    /// Delete a message from a channel.
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: channel_id, message_id
    /// Optional args: reason (audit log reason)
    #[instrument(
        skip(self, args),
        fields(
            command = "messages.delete",
            channel_id,
            message_id
        )
    )]
    async fn messages_delete(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        // Parse channel_id
        let channel_id_value = args.get("channel_id").ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "messages.delete".to_string(),
                arg_name: "channel_id".to_string(),
            })
        })?;

        let channel_id: u64 = channel_id_value
            .as_str()
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "messages.delete".to_string(),
                    arg_name: "channel_id".to_string(),
                    reason: "Must be a string".to_string(),
                })
            })?
            .parse()
            .map_err(|_| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "messages.delete".to_string(),
                    arg_name: "channel_id".to_string(),
                    reason: "Invalid Discord ID format".to_string(),
                })
            })?;

        // Parse message_id
        let message_id_value = args.get("message_id").ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "messages.delete".to_string(),
                arg_name: "message_id".to_string(),
            })
        })?;

        let message_id: u64 = message_id_value
            .as_str()
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "messages.delete".to_string(),
                    arg_name: "message_id".to_string(),
                    reason: "Must be a string".to_string(),
                })
            })?
            .parse()
            .map_err(|_| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "messages.delete".to_string(),
                    arg_name: "message_id".to_string(),
                    reason: "Invalid Discord ID format".to_string(),
                })
            })?;

        let reason = args.get("reason").and_then(|v| v.as_str());

        tracing::Span::current().record("channel_id", channel_id);
        tracing::Span::current().record("message_id", message_id);
        info!(channel_id, message_id, ?reason, "Deleting message via Discord API");

        self.http
            .delete_message(channel_id.into(), message_id.into(), reason)
            .await
            .map_err(|e| {
                error!(channel_id, message_id, error = %e, "Failed to delete message");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "messages.delete".to_string(),
                    reason: format!("Failed to delete message: {}", e),
                })
            })?;

        info!("Successfully deleted message");
        Ok(serde_json::json!({
            "deleted": true,
            "channel_id": channel_id.to_string(),
            "message_id": message_id.to_string(),
        }))
    }

    /// Execute: reactions.add
    ///
    /// Add a reaction to a message.
    ///
    /// **Security**: Low-risk write operation (easily reversible).
    ///
    /// Required args: channel_id, message_id, emoji
    #[instrument(
        skip(self, args),
        fields(
            command = "reactions.add",
            channel_id,
            message_id
        )
    )]
    async fn reactions_add(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        // Check permission
        // TODO: Security check
        // self.check_permission("reactions.add", "reactions")?;

        // Parse channel_id
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "reactions.add".to_string(),
                    arg_name: "channel_id".to_string(),
                })
            })?;
        let channel_id: u64 = channel_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "reactions.add".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        // Parse message_id
        let message_id_str = args
            .get("message_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "reactions.add".to_string(),
                    arg_name: "message_id".to_string(),
                })
            })?;
        let message_id: u64 = message_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "reactions.add".to_string(),
                arg_name: "message_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        // Parse emoji (can be Unicode emoji or custom emoji ID)
        let emoji_str = args
            .get("emoji")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "reactions.add".to_string(),
                    arg_name: "emoji".to_string(),
                })
            })?;

        tracing::Span::current().record("channel_id", channel_id);
        tracing::Span::current().record("message_id", message_id);

        info!(channel_id, message_id, emoji = emoji_str, "Adding reaction via Discord API");

        use serenity::model::channel::ReactionType;
        use serenity::model::id::{ChannelId, MessageId};

        // Try to parse as custom emoji or use as Unicode
        let reaction = if emoji_str.starts_with("custom:") {
            let emoji_id = emoji_str
                .strip_prefix("custom:")
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| {
                    BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                        command: "reactions.add".to_string(),
                        arg_name: "emoji".to_string(),
                        reason: "Custom emoji must be in format 'custom:ID'".to_string(),
                    })
                })?;
            ReactionType::Custom {
                animated: false,
                id: emoji_id.into(),
                name: Some("custom".to_string()),
            }
        } else {
            ReactionType::Unicode(emoji_str.to_string())
        };

        self.http
            .create_reaction(ChannelId::new(channel_id), MessageId::new(message_id), &reaction)
            .await
            .map_err(|e| {
                error!(channel_id, message_id, error = %e, "Failed to add reaction");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "reactions.add".to_string(),
                    reason: format!("Failed to add reaction: {}", e),
                })
            })?;

        info!("Successfully added reaction");
        Ok(serde_json::json!({
            "added": true,
            "channel_id": channel_id.to_string(),
            "message_id": message_id.to_string(),
            "emoji": emoji_str,
        }))
    }

    /// Execute: reactions.remove
    ///
    /// Remove a reaction from a message.
    ///
    /// **Security**: Low-risk write operation.
    ///
    /// Required args: channel_id, message_id, emoji
    /// Optional args: user_id (remove specific user's reaction, requires manage messages permission)
    #[instrument(
        skip(self, args),
        fields(
            command = "reactions.remove",
            channel_id,
            message_id
        )
    )]
    async fn reactions_remove(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        // Check permission
        // TODO: Security check
        // self.check_permission("reactions.remove", "reactions")?;

        // Parse channel_id
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "reactions.remove".to_string(),
                    arg_name: "channel_id".to_string(),
                })
            })?;
        let channel_id: u64 = channel_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "reactions.remove".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        // Parse message_id
        let message_id_str = args
            .get("message_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "reactions.remove".to_string(),
                    arg_name: "message_id".to_string(),
                })
            })?;
        let message_id: u64 = message_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "reactions.remove".to_string(),
                arg_name: "message_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        // Parse emoji
        let emoji_str = args
            .get("emoji")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "reactions.remove".to_string(),
                    arg_name: "emoji".to_string(),
                })
            })?;

        tracing::Span::current().record("channel_id", channel_id);
        tracing::Span::current().record("message_id", message_id);

        info!(channel_id, message_id, emoji = emoji_str, "Removing reaction via Discord API");

        use serenity::model::channel::ReactionType;
        use serenity::model::id::{ChannelId, MessageId};

        // Parse reaction type
        let reaction = if emoji_str.starts_with("custom:") {
            let emoji_id = emoji_str
                .strip_prefix("custom:")
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| {
                    BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                        command: "reactions.remove".to_string(),
                        arg_name: "emoji".to_string(),
                        reason: "Custom emoji must be in format 'custom:ID'".to_string(),
                    })
                })?;
            ReactionType::Custom {
                animated: false,
                id: emoji_id.into(),
                name: Some("custom".to_string()),
            }
        } else {
            ReactionType::Unicode(emoji_str.to_string())
        };

        // Parse user_id (required for removal)
        let user_id_str = args.get("user_id").and_then(|v| v.as_str()).ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "reactions.remove".to_string(),
                arg_name: "user_id".to_string(),
            })
        })?;
        
        let user_id: u64 = user_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "reactions.remove".to_string(),
                arg_name: "user_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        use serenity::model::id::UserId;
        self.http
            .delete_reaction(
                ChannelId::new(channel_id),
                MessageId::new(message_id),
                UserId::new(user_id),
                &reaction,
            )
            .await
            .map_err(|e| {
                error!(channel_id, message_id, user_id, error = %e, "Failed to remove reaction");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "reactions.remove".to_string(),
                    reason: format!("Failed to remove reaction: {}", e),
                })
            })?;

        info!("Successfully removed reaction");
        Ok(serde_json::json!({
            "removed": true,
            "channel_id": channel_id.to_string(),
            "message_id": message_id.to_string(),
            "emoji": emoji_str,
        }))
    }

    /// Execute: channels.edit
    ///
    /// Edit channel properties.
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: channel_id
    /// Optional args: name, topic, nsfw, position, bitrate (voice), user_limit (voice)
    #[instrument(
        skip(self, args),
        fields(
            command = "channels.edit",
            channel_id
        )
    )]
    async fn channels_edit(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        // Check permission
        // TODO: Security check
        // self.check_permission("channels.edit", "channel")?;

        // Parse channel_id
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "channels.edit".to_string(),
                    arg_name: "channel_id".to_string(),
                })
            })?;
        let channel_id: u64 = channel_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "channels.edit".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        tracing::Span::current().record("channel_id", channel_id);

        info!(channel_id, "Editing channel via Discord API");

        use serenity::builder::EditChannel;
        use serenity::model::id::ChannelId;

        let mut builder = EditChannel::new();

        if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
            builder = builder.name(name);
        }
        if let Some(topic) = args.get("topic").and_then(|v| v.as_str()) {
            builder = builder.topic(topic);
        }
        if let Some(nsfw) = args.get("nsfw").and_then(|v| v.as_bool()) {
            builder = builder.nsfw(nsfw);
        }
        if let Some(position) = args.get("position").and_then(|v| v.as_u64()) {
            builder = builder.position(position as u16);
        }
        if let Some(bitrate) = args.get("bitrate").and_then(|v| v.as_u64()) {
            builder = builder.bitrate(bitrate as u32);
        }
        if let Some(user_limit) = args.get("user_limit").and_then(|v| v.as_u64()) {
            builder = builder.user_limit(user_limit as u32);
        }

        let channel = self
            .http
            .edit_channel(ChannelId::new(channel_id), &builder, None)
            .await
            .map_err(|e| {
                error!(channel_id, error = %e, "Failed to edit channel");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "channels.edit".to_string(),
                    reason: format!("Failed to edit channel: {}", e),
                })
            })?;

        info!(channel_id, "Successfully edited channel");
        Ok(serde_json::json!({
            "id": channel.id.to_string(),
            "name": channel.name,
            "type": format!("{:?}", channel.kind),
            "position": channel.position,
            "topic": channel.topic,
            "nsfw": channel.nsfw,
        }))
    }

    /// Execute: members.kick
    ///
    /// Kick a member from the server.
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: guild_id, user_id
    /// Optional args: reason (audit log reason)
    #[instrument(
        skip(self, args),
        fields(
            command = "members.kick",
            guild_id,
            user_id
        )
    )]
    async fn members_kick(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id = Self::parse_guild_id("members.kick", args)?;

        // Parse user_id
        let user_id_value = args.get("user_id").ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: "members.kick".to_string(),
                arg_name: "user_id".to_string(),
            })
        })?;

        let user_id: u64 = user_id_value
            .as_str()
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "members.kick".to_string(),
                    arg_name: "user_id".to_string(),
                    reason: "Must be a string".to_string(),
                })
            })?
            .parse()
            .map_err(|_| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "members.kick".to_string(),
                    arg_name: "user_id".to_string(),
                    reason: "Invalid Discord ID format".to_string(),
                })
            })?;

        let reason = args.get("reason").and_then(|v| v.as_str());

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("user_id", user_id);
        info!(guild_id = %guild_id, user_id, ?reason, "Kicking member via Discord API");

        self.http
            .kick_member(guild_id, user_id.into(), reason)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, user_id, error = %e, "Failed to kick member");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "members.kick".to_string(),
                    reason: format!("Failed to kick member: {}", e),
                })
            })?;

        info!("Successfully kicked member");
        Ok(serde_json::json!({
            "kicked": true,
            "guild_id": guild_id.to_string(),
            "user_id": user_id.to_string(),
        }))
    }

    /// Execute: members.timeout
    ///
    /// Timeout a member (prevents them from sending messages/joining voice).
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: guild_id, user_id, duration_seconds
    /// Optional args: reason
    #[instrument(
        skip(self, args),
        fields(
            command = "members.timeout",
            guild_id,
            user_id,
            duration_seconds
        )
    )]
    async fn members_timeout(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id = Self::parse_guild_id("members.timeout", args)?;
        
        // Check permission

        // Parse user_id
        let user_id_str = args
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "members.timeout".to_string(),
                    arg_name: "user_id".to_string(),
                })
            })?;
        let user_id: u64 = user_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "members.timeout".to_string(),
                arg_name: "user_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        // Parse duration_seconds
        let duration_seconds = args
            .get("duration_seconds")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "members.timeout".to_string(),
                    arg_name: "duration_seconds".to_string(),
                })
            })?;

        // Discord timeout limit is 28 days (2419200 seconds)
        if duration_seconds > 2419200 {
            return Err(BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "members.timeout".to_string(),
                arg_name: "duration_seconds".to_string(),
                reason: "Timeout duration cannot exceed 28 days (2419200 seconds)".to_string(),
            }));
        }

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("user_id", user_id);
        tracing::Span::current().record("duration_seconds", duration_seconds);

        info!(guild_id = %guild_id, user_id, duration_seconds, "Timing out member via Discord API");

        use serenity::builder::EditMember;
        use serenity::model::id::UserId;
        use serenity::model::Timestamp;

        // Calculate timeout end time
        let timeout_until = Timestamp::now().unix_timestamp() + duration_seconds as i64;
        let timeout_timestamp = Timestamp::from_unix_timestamp(timeout_until)
            .map_err(|e| {
                error!("Failed to create timeout timestamp: {}", e);
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "members.timeout".to_string(),
                    arg_name: "duration_seconds".to_string(),
                    reason: format!("Invalid duration: {}", e),
                })
            })?;

        let builder = EditMember::new().disable_communication_until(timeout_timestamp.to_string());

        self.http
            .edit_member(guild_id, UserId::new(user_id), &builder, None)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, user_id, error = %e, "Failed to timeout member");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "members.timeout".to_string(),
                    reason: format!("Failed to timeout member: {}", e),
                })
            })?;

        info!("Successfully timed out member");
        Ok(serde_json::json!({
            "timed_out": true,
            "guild_id": guild_id.to_string(),
            "user_id": user_id.to_string(),
            "duration_seconds": duration_seconds,
            "timeout_until": timeout_until,
        }))
    }

    /// Execute: members.unban
    ///
    /// Unban a member (remove ban).
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: guild_id, user_id
    #[instrument(
        skip(self, args),
        fields(
            command = "members.unban",
            guild_id,
            user_id
        )
    )]
    async fn members_unban(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id = Self::parse_guild_id("members.unban", args)?;
        
        // Check permission

        // Parse user_id
        let user_id_str = args
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "members.unban".to_string(),
                    arg_name: "user_id".to_string(),
                })
            })?;
        let user_id: u64 = user_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "members.unban".to_string(),
                arg_name: "user_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("user_id", user_id);

        info!(guild_id = %guild_id, user_id, "Unbanning member via Discord API");

        use serenity::model::id::UserId;
        self.http
            .remove_ban(guild_id, UserId::new(user_id), None)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, user_id, error = %e, "Failed to unban member");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "members.unban".to_string(),
                    reason: format!("Failed to unban member: {}", e),
                })
            })?;

        info!("Successfully unbanned member");
        Ok(serde_json::json!({
            "unbanned": true,
            "guild_id": guild_id.to_string(),
            "user_id": user_id.to_string(),
        }))
    }

    /// Execute: roles.assign
    ///
    /// Assign a role to a member.
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: guild_id, user_id, role_id
    #[instrument(
        skip(self, args),
        fields(
            command = "roles.assign",
            guild_id,
            user_id,
            role_id
        )
    )]
    async fn roles_assign(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id = Self::parse_guild_id("roles.assign", args)?;
        
        // Check permission

        // Parse user_id
        let user_id_str = args
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "roles.assign".to_string(),
                    arg_name: "user_id".to_string(),
                })
            })?;
        let user_id: u64 = user_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "roles.assign".to_string(),
                arg_name: "user_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        // Parse role_id
        let role_id_str = args
            .get("role_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "roles.assign".to_string(),
                    arg_name: "role_id".to_string(),
                })
            })?;
        let role_id: u64 = role_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "roles.assign".to_string(),
                arg_name: "role_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("user_id", user_id);
        tracing::Span::current().record("role_id", role_id);

        info!(guild_id = %guild_id, user_id, role_id, "Assigning role to member via Discord API");

        use serenity::model::id::{UserId, RoleId};
        self.http
            .add_member_role(guild_id, UserId::new(user_id), RoleId::new(role_id), None)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, user_id, role_id, error = %e, "Failed to assign role");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "roles.assign".to_string(),
                    reason: format!("Failed to assign role: {}", e),
                })
            })?;

        info!("Successfully assigned role to member");
        Ok(serde_json::json!({
            "assigned": true,
            "guild_id": guild_id.to_string(),
            "user_id": user_id.to_string(),
            "role_id": role_id.to_string(),
        }))
    }

    /// Execute: roles.remove
    ///
    /// Remove a role from a member.
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: guild_id, user_id, role_id
    #[instrument(
        skip(self, args),
        fields(
            command = "roles.remove",
            guild_id,
            user_id,
            role_id
        )
    )]
    async fn roles_remove(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id = Self::parse_guild_id("roles.remove", args)?;
        
        // Check permission

        // Parse user_id
        let user_id_str = args
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "roles.remove".to_string(),
                    arg_name: "user_id".to_string(),
                })
            })?;
        let user_id: u64 = user_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "roles.remove".to_string(),
                arg_name: "user_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        // Parse role_id
        let role_id_str = args
            .get("role_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "roles.remove".to_string(),
                    arg_name: "role_id".to_string(),
                })
            })?;
        let role_id: u64 = role_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "roles.remove".to_string(),
                arg_name: "role_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("user_id", user_id);
        tracing::Span::current().record("role_id", role_id);

        info!(guild_id = %guild_id, user_id, role_id, "Removing role from member via Discord API");

        use serenity::model::id::{UserId, RoleId};
        self.http
            .remove_member_role(guild_id, UserId::new(user_id), RoleId::new(role_id), None)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, user_id, role_id, error = %e, "Failed to remove role");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "roles.remove".to_string(),
                    reason: format!("Failed to remove role: {}", e),
                })
            })?;

        info!("Successfully removed role from member");
        Ok(serde_json::json!({
            "removed": true,
            "guild_id": guild_id.to_string(),
            "user_id": user_id.to_string(),
            "role_id": role_id.to_string(),
        }))
    }

    /// Execute: roles.edit
    ///
    /// Edit role properties.
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: guild_id, role_id
    /// Optional args: name, color, hoist, mentionable, permissions
    #[instrument(
        skip(self, args),
        fields(
            command = "roles.edit",
            guild_id,
            role_id
        )
    )]
    async fn roles_edit(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id = Self::parse_guild_id("roles.edit", args)?;
        
        // Check permission

        // Parse role_id
        let role_id_str = args
            .get("role_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "roles.edit".to_string(),
                    arg_name: "role_id".to_string(),
                })
            })?;
        let role_id: u64 = role_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "roles.edit".to_string(),
                arg_name: "role_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("role_id", role_id);

        info!(guild_id = %guild_id, role_id, "Editing role via Discord API");

        use serenity::builder::EditRole;
        use serenity::model::id::RoleId;
        
        let mut builder = EditRole::new();
        
        if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
            builder = builder.name(name);
        }
        if let Some(color) = args.get("color").and_then(|v| v.as_u64()) {
            builder = builder.colour(color as u32);
        }
        if let Some(hoist) = args.get("hoist").and_then(|v| v.as_bool()) {
            builder = builder.hoist(hoist);
        }
        if let Some(mentionable) = args.get("mentionable").and_then(|v| v.as_bool()) {
            builder = builder.mentionable(mentionable);
        }

        let role = self
            .http
            .edit_role(guild_id, RoleId::new(role_id), &builder, None)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, role_id, error = %e, "Failed to edit role");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "roles.edit".to_string(),
                    reason: format!("Failed to edit role: {}", e),
                })
            })?;

        info!(role_id, "Successfully edited role");
        Ok(serde_json::json!({
            "id": role.id.to_string(),
            "name": role.name,
            "color": role.colour.0,
            "hoist": role.hoist,
            "position": role.position,
            "permissions": role.permissions.bits(),
            "mentionable": role.mentionable,
        }))
    }

    /// Execute: roles.delete
    ///
    /// Delete a role.
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: guild_id, role_id
    #[instrument(
        skip(self, args),
        fields(
            command = "roles.delete",
            guild_id,
            role_id
        )
    )]
    async fn roles_delete(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id = Self::parse_guild_id("roles.delete", args)?;
        
        // Check permission

        // Parse role_id
        let role_id_str = args
            .get("role_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "roles.delete".to_string(),
                    arg_name: "role_id".to_string(),
                })
            })?;
        let role_id: u64 = role_id_str.parse().map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "roles.delete".to_string(),
                arg_name: "role_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("role_id", role_id);

        info!(guild_id = %guild_id, role_id, "Deleting role via Discord API");

        use serenity::model::id::RoleId;
        self.http
            .delete_role(guild_id, RoleId::new(role_id), None)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, role_id, error = %e, "Failed to delete role");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "roles.delete".to_string(),
                    reason: format!("Failed to delete role: {}", e),
                })
            })?;

        info!(role_id, "Successfully deleted role");
        Ok(serde_json::json!({
            "deleted": true,
            "guild_id": guild_id.to_string(),
            "role_id": role_id.to_string(),
        }))
    }

    /// Execute: roles.create
    ///
    /// Create a new role in the server.
    ///
    /// **Security**: This command MUST go through the security framework.
    ///
    /// Required args: guild_id, name
    /// Optional args: color (hex), hoist (bool), mentionable (bool), permissions (u64)
    #[instrument(
        skip(self, args),
        fields(
            command = "roles.create",
            guild_id,
            role_name
        )
    )]
    async fn roles_create(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id = Self::parse_guild_id("roles.create", args)?;

        // Parse name
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "roles.create".to_string(),
                    arg_name: "name".to_string(),
                })
            })?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("role_name", name);

        // Parse optional parameters
        let color = args.get("color").and_then(|v| v.as_u64()).map(|c| c as u32);
        let hoist = args.get("hoist").and_then(|v| v.as_bool()).unwrap_or(false);
        let mentionable = args.get("mentionable").and_then(|v| v.as_bool()).unwrap_or(false);

        info!(guild_id = %guild_id, name, color, hoist, mentionable, "Creating role via Discord API");

        use serenity::builder::EditRole;
        let mut builder = EditRole::new().name(name).hoist(hoist).mentionable(mentionable);
        
        if let Some(c) = color {
            builder = builder.colour(c);
        }

        let role = self
            .http
            .create_role(guild_id, &builder, None)
            .await
            .map_err(|e| {
                error!(guild_id = %guild_id, error = %e, "Failed to create role");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "roles.create".to_string(),
                    reason: format!("Failed to create role: {}", e),
                })
            })?;

        info!(role_id = %role.id, "Successfully created role");
        Ok(serde_json::json!({
            "id": role.id.to_string(),
            "name": role.name,
            "color": role.colour.0,
            "hoist": role.hoist,
            "position": role.position,
            "permissions": role.permissions.bits(),
            "mentionable": role.mentionable,
        }))
    }

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
        
        // Security check: require permission to send messages to this channel
        // TODO: Security check
        // self.check_permission("messages.send", channel_id_str)?;

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

        // Discord has a 2000 character limit for messages
        const MAX_MESSAGE_LENGTH: usize = 2000;
        
        if content.len() <= MAX_MESSAGE_LENGTH {
            // Send single message
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
        } else {
            // Split into multiple messages
            info!(
                content_len = content.len(),
                max_len = MAX_MESSAGE_LENGTH,
                "Message too long, splitting into multiple messages"
            );
            
            let mut messages = Vec::new();
            let mut current_pos = 0;
            
            while current_pos < content.len() {
                let end_pos = (current_pos + MAX_MESSAGE_LENGTH).min(content.len());
                let chunk = &content[current_pos..end_pos];
                
                let message = channel_id
                    .send_message(&self.http, CreateMessage::new().content(chunk).tts(tts))
                    .await
                    .map_err(|e| {
                        error!(channel_id = %channel_id, error = %e, "Failed to send message chunk");
                        BotCommandError::new(BotCommandErrorKind::ApiError {
                            command: "messages.send".to_string(),
                            reason: format!("Failed to send message chunk: {}", e),
                        })
                    })?;
                
                info!(message_id = %message.id, chunk_num = messages.len() + 1, "Successfully sent message chunk");
                messages.push(message);
                current_pos = end_pos;
            }
            
            // Return info about the first message
            let first_message = &messages[0];
            Ok(serde_json::json!({
                "id": first_message.id.to_string(),
                "channel_id": first_message.channel_id.to_string(),
                "content": first_message.content,
                "timestamp": first_message.timestamp.to_rfc3339(),
                "tts": first_message.tts,
                "split_messages": messages.len(),
            }))
        }
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
        
        // Security check: require permission to create channels in this guild

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
        
        // Security check: require permission to delete this channel
        // TODO: Security check
        // self.check_permission("channels.delete", channel_id_str)?;

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
        
        // Security check: require permission to ban members in this guild

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

    /// Execute: messages.pin
    ///
    /// Pin a message in a channel.
    ///
    /// Required args: channel_id, message_id
    /// Security: Requires Write permission on Channel
    #[instrument(
        skip(self, args),
        fields(
            command = "messages.pin",
            channel_id,
            message_id
        )
    )]
    async fn messages_pin(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::model::id::{ChannelId, MessageId};

        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "messages.pin".to_string(),
                    arg_name: "channel_id".to_string(),
                })
            })?;

        let channel_id = channel_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.pin".to_string(),
                arg_name: "channel_id".to_string(),
                reason: format!("Invalid channel ID format: {}", e),
            })
        })?;

        // TODO: Security check
        // self.check_permission("messages.pin", channel_id_str)?;

        let message_id_str = args
            .get("message_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "messages.pin".to_string(),
                    arg_name: "message_id".to_string(),
                })
            })?;

        let message_id = message_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.pin".to_string(),
                arg_name: "message_id".to_string(),
                reason: format!("Invalid message ID format: {}", e),
            })
        })?;

        info!("Pinning message");

        self.http
            .pin_message(ChannelId::new(channel_id), MessageId::new(message_id), None)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to pin message");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "messages.pin".to_string(),
                    reason: format!("Failed to pin message: {}", e),
                })
            })?;

        info!("Successfully pinned message");

        Ok(serde_json::json!({
            "channel_id": channel_id_str,
            "message_id": message_id_str,
            "pinned": true,
        }))
    }

    /// Execute: messages.unpin
    ///
    /// Unpin a message in a channel.
    ///
    /// Required args: channel_id, message_id
    /// Security: Requires Write permission on Channel
    #[instrument(
        skip(self, args),
        fields(
            command = "messages.unpin",
            channel_id,
            message_id
        )
    )]
    async fn messages_unpin(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::model::id::{ChannelId, MessageId};

        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "messages.unpin".to_string(),
                    arg_name: "channel_id".to_string(),
                })
            })?;

        let channel_id = channel_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.unpin".to_string(),
                arg_name: "channel_id".to_string(),
                reason: format!("Invalid channel ID format: {}", e),
            })
        })?;

        // TODO: Security check
        // self.check_permission("messages.unpin", channel_id_str)?;

        let message_id_str = args
            .get("message_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "messages.unpin".to_string(),
                    arg_name: "message_id".to_string(),
                })
            })?;

        let message_id = message_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "messages.unpin".to_string(),
                arg_name: "message_id".to_string(),
                reason: format!("Invalid message ID format: {}", e),
            })
        })?;

        info!("Unpinning message");

        self.http
            .unpin_message(ChannelId::new(channel_id), MessageId::new(message_id), None)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to unpin message");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "messages.unpin".to_string(),
                    reason: format!("Failed to unpin message: {}", e),
                })
            })?;

        info!("Successfully unpinned message");

        Ok(serde_json::json!({
            "channel_id": channel_id_str,
            "message_id": message_id_str,
            "pinned": false,
        }))
    }

    /// Execute: members.edit
    ///
    /// Edit member properties (nickname, roles, mute, deafen).
    ///
    /// Required args: guild_id, user_id
    /// Optional args: nickname, mute, deafen, roles (array of role IDs)
    /// Security: Requires Write permission on Member
    #[instrument(
        skip(self, args),
        fields(
            command = "members.edit",
            guild_id,
            user_id
        )
    )]
    async fn members_edit(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::builder::EditMember;
        use serenity::model::id::{RoleId, UserId};

        let guild_id = Self::parse_guild_id("members.edit", args)?;

        let user_id_str = args
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "members.edit".to_string(),
                    arg_name: "user_id".to_string(),
                })
            })?;

        let user_id = user_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "members.edit".to_string(),
                arg_name: "user_id".to_string(),
                reason: format!("Invalid user ID format: {}", e),
            })
        })?;

        // TODO: Security check
        // self.check_permission("members.edit", user_id_str)?;

        let user_id = UserId::new(user_id);

        info!(guild_id = %guild_id, user_id = %user_id, "Editing member");

        let builder = EditMember::new();
        let mut builder = builder;
        let mut changes = Vec::new();

        if let Some(nickname) = args.get("nickname").and_then(|v| v.as_str()) {
            builder = builder.nickname(nickname);
            changes.push(format!("nickname={}", nickname));
        }

        if let Some(mute) = args.get("mute").and_then(|v| v.as_bool()) {
            builder = builder.mute(mute);
            changes.push(format!("mute={}", mute));
        }

        if let Some(deafen) = args.get("deafen").and_then(|v| v.as_bool()) {
            builder = builder.deafen(deafen);
            changes.push(format!("deafen={}", deafen));
        }

        if let Some(roles) = args.get("roles").and_then(|v| v.as_array()) {
            let role_ids: Result<Vec<RoleId>, _> = roles
                .iter()
                .map(|r| {
                    r.as_str()
                        .and_then(|s| s.parse::<u64>().ok())
                        .map(RoleId::new)
                        .ok_or_else(|| {
                            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                                command: "members.edit".to_string(),
                                arg_name: "roles".to_string(),
                                reason: "Invalid role ID format".to_string(),
                            })
                        })
                })
                .collect();

            let role_ids = role_ids?;
            changes.push(format!("roles={:?}", role_ids));
            builder = builder.roles(&role_ids);
        }

        debug!(changes = ?changes, "Applying member changes");

        guild_id
            .edit_member(&self.http, user_id, builder)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to edit member");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "members.edit".to_string(),
                    reason: format!("Failed to edit member: {}", e),
                })
            })?;

        info!("Successfully edited member");

        Ok(serde_json::json!({
            "guild_id": guild_id.to_string(),
            "user_id": user_id_str,
            "changes": changes,
        }))
    }

    /// Execute: members.remove_timeout
    ///
    /// Remove timeout from a member.
    ///
    /// Required args: guild_id, user_id
    /// Security: Requires Write permission on Member
    #[instrument(
        skip(self, args),
        fields(
            command = "members.remove_timeout",
            guild_id,
            user_id
        )
    )]
    async fn members_remove_timeout(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::builder::EditMember;
        use serenity::model::id::UserId;

        let guild_id = Self::parse_guild_id("members.remove_timeout", args)?;

        let user_id_str = args
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "members.remove_timeout".to_string(),
                    arg_name: "user_id".to_string(),
                })
            })?;

        let user_id = user_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "members.remove_timeout".to_string(),
                arg_name: "user_id".to_string(),
                reason: format!("Invalid user ID format: {}", e),
            })
        })?;

        // TODO: Security check
        // self.check_permission("members.remove_timeout", user_id_str)?;

        let user_id = UserId::new(user_id);

        info!(guild_id = %guild_id, user_id = %user_id, "Removing member timeout");

        let builder = EditMember::new().disable_communication_until_datetime(serenity::model::Timestamp::now());

        guild_id
            .edit_member(&self.http, user_id, builder)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to remove timeout");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "members.remove_timeout".to_string(),
                    reason: format!("Failed to remove timeout: {}", e),
                })
            })?;

        info!("Successfully removed member timeout");

        Ok(serde_json::json!({
            "guild_id": guild_id.to_string(),
            "user_id": user_id_str,
            "timeout_removed": true,
        }))
    }

    /// Execute: channels.create_invite
    ///
    /// Create an invite link for a channel.
    ///
    /// Required args: channel_id
    /// Optional args: max_age (seconds, 0 = never), max_uses (0 = unlimited), temporary (bool)
    /// Security: Requires Write permission on Channel
    #[instrument(
        skip(self, args),
        fields(
            command = "channels.create_invite",
            channel_id
        )
    )]
    async fn channels_create_invite(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::builder::CreateInvite;
        use serenity::model::id::ChannelId;

        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "channels.create_invite".to_string(),
                    arg_name: "channel_id".to_string(),
                })
            })?;

        let channel_id = channel_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "channels.create_invite".to_string(),
                arg_name: "channel_id".to_string(),
                reason: format!("Invalid channel ID format: {}", e),
            })
        })?;

        // TODO: Security check
        // self.check_permission("channels.create_invite", channel_id_str)?;

        let channel_id = ChannelId::new(channel_id);

        info!(channel_id = %channel_id, "Creating invite");

        let mut builder = CreateInvite::new();

        if let Some(max_age) = args.get("max_age").and_then(|v| v.as_u64()) {
            builder = builder.max_age(max_age as u32);
        }

        if let Some(max_uses) = args.get("max_uses").and_then(|v| v.as_u64()) {
            builder = builder.max_uses(max_uses as u8);
        }

        if let Some(temporary) = args.get("temporary").and_then(|v| v.as_bool()) {
            builder = builder.temporary(temporary);
        }

        let invite = channel_id
            .create_invite(&self.http, builder)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to create invite");
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "channels.create_invite".to_string(),
                    reason: format!("Failed to create invite: {}", e),
                })
            })?;

        info!(code = %invite.code, "Successfully created invite");

        Ok(serde_json::json!({
            "code": invite.code,
            "url": format!("https://discord.gg/{}", invite.code),
            "channel_id": invite.channel.id.to_string(),
            "max_age": invite.max_age,
            "max_uses": invite.max_uses,
            "temporary": invite.temporary,
        }))
    }

    /// Execute: channels.typing
    ///
    /// Trigger typing indicator in a channel.
    ///
    /// Required args: channel_id
    /// Security: Low-risk write operation (typing indicator)
    #[instrument(
        skip(self, args),
        fields(
            command = "channels.typing",
            channel_id
        )
    )]
    async fn channels_typing(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        use serenity::model::id::ChannelId;

        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "channels.typing".to_string(),
                    arg_name: "channel_id".to_string(),
                })
            })?;

        let channel_id = channel_id_str.parse::<u64>().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "channels.typing".to_string(),
                arg_name: "channel_id".to_string(),
                reason: format!("Invalid channel ID format: {}", e),
            })
        })?;

        // Typing is low-risk, but still requires permission checker
        // TODO: Security check
        // self.check_permission("channels.typing", channel_id_str)?;

        let channel_id = ChannelId::new(channel_id);

        debug!(channel_id = %channel_id, "Triggering typing indicator");

        channel_id.broadcast_typing(&self.http).await.map_err(|e| {
            error!(error = %e, "Failed to trigger typing");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "channels.typing".to_string(),
                reason: format!("Failed to trigger typing: {}", e),
            })
        })?;

        debug!("Successfully triggered typing indicator");

        Ok(serde_json::json!({
            "channel_id": channel_id_str,
            "typing": true,
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
            "messages.get" => self.messages_get(args).await?,
            "messages.list" => self.messages_list(args).await?,
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
            "messages.edit" => self.messages_edit(args).await?,
            "messages.delete" => self.messages_delete(args).await?,
            "reactions.add" => self.reactions_add(args).await?,
            "reactions.remove" => self.reactions_remove(args).await?,
            "channels.create" => self.channels_create(args).await?,
            "channels.edit" => self.channels_edit(args).await?,
            "channels.delete" => self.channels_delete(args).await?,
            "members.ban" => self.members_ban(args).await?,
            "members.kick" => self.members_kick(args).await?,
            "members.timeout" => self.members_timeout(args).await?,
            "members.unban" => self.members_unban(args).await?,
            "roles.create" => self.roles_create(args).await?,
            "roles.assign" => self.roles_assign(args).await?,
            "roles.remove" => self.roles_remove(args).await?,
            "roles.edit" => self.roles_edit(args).await?,
            "roles.delete" => self.roles_delete(args).await?,
            "messages.pin" => self.messages_pin(args).await?,
            "messages.unpin" => self.messages_unpin(args).await?,
            "members.edit" => self.members_edit(args).await?,
            "members.remove_timeout" => self.members_remove_timeout(args).await?,
            "channels.create_invite" => self.channels_create_invite(args).await?,
            "channels.typing" => self.channels_typing(args).await?,
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
                | "messages.get"
                | "messages.list"
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
                | "messages.edit"
                | "messages.delete"
                | "reactions.add"
                | "reactions.remove"
                | "channels.create"
                | "channels.edit"
                | "channels.delete"
                | "members.ban"
                | "members.kick"
                | "members.timeout"
                | "members.unban"
                | "roles.create"
                | "roles.assign"
                | "roles.remove"
                | "roles.edit"
                | "roles.delete"
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
            "messages.get".to_string(),
            "messages.list".to_string(),
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
            "messages.edit".to_string(),
            "messages.delete".to_string(),
            "messages.pin".to_string(),
            "messages.unpin".to_string(),
            "channels.create".to_string(),
            "channels.edit".to_string(),
            "channels.delete".to_string(),
            "channels.create_invite".to_string(),
            "channels.typing".to_string(),
            "members.ban".to_string(),
            "members.kick".to_string(),
            "members.timeout".to_string(),
            "members.unban".to_string(),
            "members.edit".to_string(),
            "members.remove_timeout".to_string(),
            "roles.create".to_string(),
            "roles.assign".to_string(),
            "roles.remove".to_string(),
            "roles.edit".to_string(),
            "roles.delete".to_string(),
            "reactions.add".to_string(),
            "reactions.remove".to_string(),
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
            "messages.get" => Some(
                "Get a specific message from a channel\n\
                 Required arguments: channel_id, message_id"
                    .to_string(),
            ),
            "messages.list" => Some(
                "List message history from a channel\n\
                 Required arguments: channel_id\n\
                 Optional arguments: limit (default 50, max 100)"
                    .to_string(),
            ),
            "messages.send" => Some(
                "Send a message to a channel (requires security framework)\n\
                 Required arguments: guild_id, channel_id, content\n\
                 Optional arguments: tts (default false)"
                    .to_string(),
            ),
            "messages.edit" => Some(
                "Edit an existing message (requires security framework)\n\
                 Required arguments: channel_id, message_id, content"
                    .to_string(),
            ),
            "messages.delete" => Some(
                "Delete a message (requires security framework)\n\
                 Required arguments: channel_id, message_id\n\
                 Optional arguments: reason"
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
            "members.kick" => Some(
                "Kick a member from the server (requires security framework)\n\
                 Required arguments: guild_id, user_id\n\
                 Optional arguments: reason"
                    .to_string(),
            ),
            "members.ban" => Some(
                "Ban a member (requires security framework and approval)\n\
                 Required arguments: guild_id, user_id\n\
                 Optional arguments: delete_message_days (0-7), reason"
                    .to_string(),
            ),
            "roles.create" => Some(
                "Create a new role in the server (requires security framework)\n\
                 Required arguments: guild_id, name\n\
                 Optional arguments: color (hex), hoist (bool), mentionable (bool), permissions (u64)"
                    .to_string(),
            ),
            "messages.pin" => Some(
                "Pin a message in a channel (requires security framework)\n\
                 Required arguments: channel_id, message_id"
                    .to_string(),
            ),
            "messages.unpin" => Some(
                "Unpin a message in a channel (requires security framework)\n\
                 Required arguments: channel_id, message_id"
                    .to_string(),
            ),
            "members.edit" => Some(
                "Edit member properties (requires security framework)\n\
                 Required arguments: guild_id, user_id\n\
                 Optional arguments: nickname, mute (bool), deafen (bool), roles (array of role IDs)"
                    .to_string(),
            ),
            "members.remove_timeout" => Some(
                "Remove timeout from a member (requires security framework)\n\
                 Required arguments: guild_id, user_id"
                    .to_string(),
            ),
            "channels.create_invite" => Some(
                "Create an invite link for a channel (requires security framework)\n\
                 Required arguments: channel_id\n\
                 Optional arguments: max_age (seconds, 0 = never), max_uses (0 = unlimited), temporary (bool)"
                    .to_string(),
            ),
            "channels.typing" => Some(
                "Trigger typing indicator in a channel (low-risk write)\n\
                 Required arguments: channel_id"
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
