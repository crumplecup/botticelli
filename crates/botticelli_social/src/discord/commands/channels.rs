//! Channel commands for Discord guilds.
//!
//! This module handles channel operations such as listing, creating,
//! editing, deleting channels, and managing invites.

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{
    ChannelId, ChannelType, CreateChannel, CreateInvite, EditChannel, GuildId, Http,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};

/// Channel command namespace.
///
/// This zero-sized type provides a clean namespace for channel-related
/// Discord commands without polluting the module with free functions.
#[derive(Debug, Clone, Copy)]
pub(super) struct Channels;

impl Channels {
    /// Execute: channels.list
    ///
    /// List all channels in a guild.
    #[instrument(skip(http, args), fields(guild_id, channel_count))]
    pub async fn list(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id_str = args
            .get("guild_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("guild_id"))?;

        let guild_id = parse_guild_id(guild_id_str)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        debug!(guild_id = %guild_id, "Fetching channels");

        let channels = http.get_channels(guild_id).await.map_err(|e| {
            error!(error = %e, "Failed to fetch channels");
            BotCommandError::from_api_error("channels.list", e)
        })?;

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

        tracing::Span::current().record("channel_count", channels_json.len());
        info!(
            channel_count = channels_json.len(),
            "Successfully retrieved channels"
        );
        Ok(serde_json::json!(channels_json))
    }

    /// Execute: channels.get
    ///
    /// Get specific channel details.
    #[instrument(skip(http, args), fields(guild_id, channel_id))]
    pub async fn get(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id_str = args
            .get("guild_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("guild_id"))?;
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("channel_id"))?;

        let guild_id = parse_guild_id(guild_id_str)?;
        let channel_id = parse_channel_id(channel_id_str)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("channel_id", channel_id.get());
        debug!(guild_id = %guild_id, channel_id = %channel_id, "Fetching channel");

        let channels = http.get_channels(guild_id).await.map_err(|e| {
            error!(error = %e, "Failed to fetch channels");
            BotCommandError::from_api_error("channels.get", e)
        })?;

        let channel = channels
            .into_iter()
            .find(|c| c.id == channel_id)
            .ok_or_else(|| {
                error!(guild_id = %guild_id, channel_id = %channel_id, "Channel not found");
                BotCommandError::new(BotCommandErrorKind::ResourceNotFound {
                    command: "channels.get".to_string(),
                    resource_type: "channel".to_string(),
                })
            })?;

        info!(channel_id = %channel_id, "Successfully retrieved channel details");
        Ok(serde_json::json!({
            "id": channel.id.to_string(),
            "name": channel.name,
            "type": format!("{:?}", channel.kind),
            "position": channel.position,
            "topic": channel.topic,
            "nsfw": channel.nsfw,
            "parent_id": channel.parent_id.map(|id| id.to_string()),
            "rate_limit_per_user": channel.rate_limit_per_user,
            "bitrate": channel.bitrate,
        }))
    }

    /// Execute: channels.create
    ///
    /// Create a new channel in the guild.
    #[instrument(skip(http, args), fields(guild_id, name, kind))]
    pub async fn create(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id_str = args
            .get("guild_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("guild_id"))?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("name"))?;
        let kind_str = args
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("kind"))?;

        let guild_id = parse_guild_id(guild_id_str)?;
        let kind = parse_channel_type(kind_str)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("name", name);
        tracing::Span::current().record("kind", kind_str);
        info!(guild_id = %guild_id, name, kind = kind_str, "Creating channel");

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

        let channel = guild_id.create_channel(http, builder).await.map_err(|e| {
            error!(error = %e, "Failed to create channel");
            BotCommandError::from_api_error("channels.create", e)
        })?;

        info!(channel_id = %channel.id, name, "Successfully created channel");
        Ok(serde_json::json!({
            "id": channel.id.to_string(),
            "name": channel.name,
            "kind": format!("{:?}", channel.kind),
            "position": channel.position,
        }))
    }

    /// Execute: channels.edit
    ///
    /// Edit channel properties.
    #[instrument(skip(http, args), fields(channel_id))]
    pub async fn edit(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("channel_id"))?;

        let channel_id = parse_channel_id(channel_id_str)?;

        tracing::Span::current().record("channel_id", channel_id.get());
        info!(channel_id = %channel_id, "Editing channel");

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

        let channel = http
            .edit_channel(channel_id, &builder, None)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to edit channel");
                BotCommandError::from_api_error("channels.edit", e)
            })?;

        info!(channel_id = %channel_id, "Successfully edited channel");
        Ok(serde_json::json!({
            "id": channel.id.to_string(),
            "name": channel.name,
            "type": format!("{:?}", channel.kind),
            "position": channel.position,
            "topic": channel.topic,
            "nsfw": channel.nsfw,
        }))
    }

    /// Execute: channels.delete
    ///
    /// Delete a channel.
    #[instrument(skip(http, args), fields(guild_id, channel_id))]
    pub async fn delete(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id_str = args
            .get("guild_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("guild_id"))?;
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("channel_id"))?;

        let guild_id = parse_guild_id(guild_id_str)?;
        let channel_id = parse_channel_id(channel_id_str)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("channel_id", channel_id.get());
        warn!(guild_id = %guild_id, channel_id = %channel_id, "Deleting channel");

        channel_id.delete(http).await.map_err(|e| {
            error!(error = %e, "Failed to delete channel");
            BotCommandError::from_api_error("channels.delete", e)
        })?;

        info!(channel_id = %channel_id, "Successfully deleted channel");
        Ok(serde_json::json!({
            "id": channel_id.to_string(),
            "deleted": true,
        }))
    }

    /// Execute: channels.get_or_create
    ///
    /// Get a channel by name, or create it if it doesn't exist.
    #[instrument(skip(http, args), fields(guild_id, name, existed))]
    pub async fn get_or_create(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let guild_id_str = args
            .get("guild_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("guild_id"))?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("name"))?;

        let guild_id = parse_guild_id(guild_id_str)?;

        tracing::Span::current().record("guild_id", guild_id.get());
        tracing::Span::current().record("name", name);
        debug!(guild_id = %guild_id, name, "Checking if channel exists");

        let channels = http.get_channels(guild_id).await.map_err(|e| {
            error!(error = %e, "Failed to fetch channels");
            BotCommandError::from_api_error("channels.get_or_create", e)
        })?;

        if let Some(existing) = channels.iter().find(|c| c.name == name) {
            tracing::Span::current().record("existed", true);
            info!(channel_id = %existing.id, name, "Channel already exists");
            return Ok(serde_json::json!({
                "id": existing.id.to_string(),
                "name": existing.name,
                "kind": format!("{:?}", existing.kind),
                "position": existing.position,
                "existed": true,
            }));
        }

        info!(guild_id = %guild_id, name, "Channel doesn't exist, creating");

        let kind_str = args
            .get("channel_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text");
        let kind = parse_channel_type(kind_str)?;

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

        let channel = guild_id.create_channel(http, builder).await.map_err(|e| {
            error!(error = %e, "Failed to create channel");
            BotCommandError::from_api_error("channels.get_or_create", e)
        })?;

        info!(channel_id = %channel.id, name, "Successfully created channel");
        Ok(serde_json::json!({
            "id": channel.id.to_string(),
            "name": channel.name,
            "kind": format!("{:?}", channel.kind),
            "position": channel.position,
            "existed": false,
        }))
    }

    /// Execute: channels.create_invite
    ///
    /// Create an invite link for a channel.
    #[instrument(skip(http, args), fields(channel_id, max_age, max_uses))]
    pub async fn create_invite(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("channel_id"))?;

        let channel_id = parse_channel_id(channel_id_str)?;

        tracing::Span::current().record("channel_id", channel_id.get());
        info!(channel_id = %channel_id, "Creating invite");

        let mut builder = CreateInvite::new();

        if let Some(max_age) = args.get("max_age").and_then(|v| v.as_u64()) {
            tracing::Span::current().record("max_age", max_age);
            builder = builder.max_age(max_age as u32);
        }
        if let Some(max_uses) = args.get("max_uses").and_then(|v| v.as_u64()) {
            tracing::Span::current().record("max_uses", max_uses);
            builder = builder.max_uses(max_uses as u8);
        }
        if let Some(temporary) = args.get("temporary").and_then(|v| v.as_bool()) {
            builder = builder.temporary(temporary);
        }

        let invite = channel_id.create_invite(http, builder).await.map_err(|e| {
            error!(error = %e, "Failed to create invite");
            BotCommandError::from_api_error("channels.create_invite", e)
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
    #[instrument(skip(http, args), fields(channel_id))]
    pub async fn typing(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        let channel_id_str = args
            .get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing_arg_error("channel_id"))?;

        let channel_id = parse_channel_id(channel_id_str)?;

        tracing::Span::current().record("channel_id", channel_id.get());
        debug!(channel_id = %channel_id, "Triggering typing indicator");

        channel_id.broadcast_typing(http).await.map_err(|e| {
            error!(error = %e, "Failed to trigger typing");
            BotCommandError::from_api_error("channels.typing", e)
        })?;

        debug!("Successfully triggered typing indicator");
        Ok(serde_json::json!({
            "channel_id": channel_id_str,
            "typing": true,
        }))
    }
}

// Helper functions

#[instrument(skip(arg_name))]
#[track_caller]
fn missing_arg_error(arg_name: &str) -> BotCommandError {
    BotCommandError::new(BotCommandErrorKind::MissingArgument {
        command: "".to_string(),
        arg_name: arg_name.to_string(),
    })
}

#[instrument(skip(s))]
#[track_caller]
fn parse_guild_id(s: &str) -> BotCommandResult<GuildId> {
    s.parse::<u64>()
        .map(GuildId::new)
        .map_err(|e| BotCommandError::from_parse_error("guild_id", e))
}

#[instrument(skip(s))]
fn parse_channel_id(s: &str) -> BotCommandResult<ChannelId> {
    s.parse::<u64>()
        .map(ChannelId::new)
        .map_err(|e| BotCommandError::from_parse_error("channel_id", e))
}

#[instrument(skip(s))]
fn parse_channel_type(s: &str) -> BotCommandResult<ChannelType> {
    match s {
        "text" => Ok(ChannelType::Text),
        "voice" => Ok(ChannelType::Voice),
        "category" => Ok(ChannelType::Category),
        "announcement" => Ok(ChannelType::News),
        "stage" => Ok(ChannelType::Stage),
        "forum" => Ok(ChannelType::Forum),
        _ => Err(BotCommandErrorKind::InvalidArgument {
            command: "".to_string(),
            arg_name: "kind".to_string(),
            reason: format!("Invalid channel type: {}", s),
        }
        .into()),
    }
}
