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
            "server.get_stats" => self.server_get_stats(args).await?,
            "channels.list" => self.channels_list(args).await?,
            "channels.get" => self.channels_get(args).await?,
            "roles.list" => self.roles_list(args).await?,
            "members.list" => self.members_list(args).await?,
            "members.get" => self.members_get(args).await?,
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
            "server.get_stats"
                | "channels.list"
                | "channels.get"
                | "roles.list"
                | "members.list"
                | "members.get"
        )
    }

    fn supported_commands(&self) -> Vec<String> {
        vec![
            "server.get_stats".to_string(),
            "channels.list".to_string(),
            "channels.get".to_string(),
            "roles.list".to_string(),
            "members.list".to_string(),
            "members.get".to_string(),
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
        assert!(executor.supports_command("members.list"));
        assert!(executor.supports_command("members.get"));
        assert!(!executor.supports_command("unknown.command"));
    }

    #[test]
    fn test_supported_commands() {
        let token = "test_token";
        let executor = DiscordCommandExecutor::new(token);

        let commands = executor.supported_commands();
        assert_eq!(commands.len(), 6);
        assert!(commands.contains(&"server.get_stats".to_string()));
        assert!(commands.contains(&"channels.list".to_string()));
        assert!(commands.contains(&"channels.get".to_string()));
        assert!(commands.contains(&"roles.list".to_string()));
        assert!(commands.contains(&"members.list".to_string()));
        assert!(commands.contains(&"members.get".to_string()));
    }

    #[test]
    fn test_command_help() {
        let token = "test_token";
        let executor = DiscordCommandExecutor::new(token);

        assert!(executor.command_help("server.get_stats").is_some());
        assert!(executor.command_help("channels.list").is_some());
        assert!(executor.command_help("channels.get").is_some());
        assert!(executor.command_help("roles.list").is_some());
        assert!(executor.command_help("members.list").is_some());
        assert!(executor.command_help("members.get").is_some());
        assert!(executor.command_help("unknown.command").is_none());
    }

    #[test]
    fn test_platform() {
        let token = "test_token";
        let executor = DiscordCommandExecutor::new(token);

        assert_eq!(executor.platform(), "discord");
    }
}
