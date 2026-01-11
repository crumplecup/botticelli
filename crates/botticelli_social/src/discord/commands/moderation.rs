//! Moderation commands (bans, kicks, timeouts).

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{GuildId, Http, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// List banned users in a server.
///
/// Command: `bans.list`
/// Required arguments: `guild_id`
/// Optional arguments: `limit` (default 100, max 1000)
pub(super) async fn list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing guild_id argument");
    let guild_id = parse_guild_id("bans.list", args)?;

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
    let bans = http
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

/// Ban a member from the server.
///
/// Command: `members.ban`
/// Required arguments: `guild_id`, `user_id`
/// Optional arguments: `delete_message_days` (0-7, default 0)
pub(super) async fn ban(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing arguments for members.ban");
    let guild_id = parse_guild_id("members.ban", args)?;

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
        .ban(http, user_id, delete_message_days)
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

/// Unban a member from the server.
///
/// Command: `members.unban`
/// Required arguments: `guild_id`, `user_id`
pub(super) async fn unban(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id = parse_guild_id("members.unban", args)?;

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

    http
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

/// Kick a member from the server.
///
/// Command: `members.kick`
/// Required arguments: `guild_id`, `user_id`
/// Optional arguments: `reason`
pub(super) async fn kick(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id = parse_guild_id("members.kick", args)?;

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

    http
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

/// Parse guild_id from command arguments.
fn parse_guild_id(command: &str, args: &HashMap<String, JsonValue>) -> BotCommandResult<GuildId> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::MissingArgument {
                command: command.to_string(),
                arg_name: "guild_id".to_string(),
            })
        })?;

    let guild_id = guild_id_str.parse::<u64>().map_err(|e| {
        BotCommandError::new(BotCommandErrorKind::InvalidArgument {
            command: command.to_string(),
            arg_name: "guild_id".to_string(),
            reason: format!("Invalid guild_id format: {}", e),
        })
    })?;

    Ok(GuildId::new(guild_id))
}
