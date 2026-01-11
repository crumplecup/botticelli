//! Member commands for Discord guilds (non-moderation).
//!
//! This module handles member operations such as listing, getting details,
//! editing member properties, and managing timeouts.

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{EditMember, GuildId, Http, RoleId, Timestamp, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Execute: members.list
///
/// List members in a guild.
#[instrument(skip(http, args), fields(guild_id, limit, member_count))]
pub(super) async fn list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;

    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(100)
        .min(1000); // Discord's max is 1000

    tracing::Span::current().record("guild_id", guild_id.get());
    tracing::Span::current().record("limit", limit);
    debug!(guild_id = %guild_id, limit, "Fetching guild members");

    let members = http
        .get_guild_members(guild_id, Some(limit), None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to fetch members");
            BotCommandError::from_api_error("members.list", e)
        })?;

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

    tracing::Span::current().record("member_count", members_json.len());
    info!(member_count = members_json.len(), "Successfully retrieved guild members");
    Ok(serde_json::json!(members_json))
}

/// Execute: members.get
///
/// Get specific member details.
#[instrument(skip(http, args), fields(guild_id, user_id))]
pub(super) async fn get(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let user_id_str = args
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("user_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;
    let user_id = parse_user_id(user_id_str)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    tracing::Span::current().record("user_id", user_id.get());
    debug!(guild_id = %guild_id, user_id = %user_id, "Fetching member");

    let member = http.get_member(guild_id, user_id).await.map_err(|e| {
        error!(error = %e, "Failed to fetch member");
        BotCommandError::from_api_error("members.get", e)
    })?;

    let roles: Vec<String> = member
        .roles
        .iter()
        .map(|role_id| role_id.to_string())
        .collect();

    info!(user_id = %user_id, "Successfully retrieved member details");
    Ok(serde_json::json!({
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
    }))
}

/// Execute: members.edit
///
/// Edit member properties.
#[instrument(skip(http, args), fields(guild_id, user_id))]
pub(super) async fn edit(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let user_id_str = args
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("user_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;
    let user_id = parse_user_id(user_id_str)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    tracing::Span::current().record("user_id", user_id.get());
    info!(guild_id = %guild_id, user_id = %user_id, "Editing member");

    let mut builder = EditMember::new();
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
        .edit_member(http, user_id, builder)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to edit member");
            BotCommandError::from_api_error("members.edit", e)
        })?;

    info!("Successfully edited member");
    Ok(serde_json::json!({
        "guild_id": guild_id.to_string(),
        "user_id": user_id_str,
        "changes": changes,
    }))
}

/// Execute: members.timeout
///
/// Apply timeout to a member.
#[instrument(skip(http, args), fields(guild_id, user_id, duration_seconds))]
pub(super) async fn timeout(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let user_id_str = args
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("user_id"))?;
    let duration_seconds = args
        .get("duration_seconds")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| missing_arg_error("duration_seconds"))?;

    let guild_id = parse_guild_id(guild_id_str)?;
    let user_id = parse_user_id(user_id_str)?;

    // Discord timeout limit is 28 days (2419200 seconds)
    if duration_seconds > 2419200 {
        return Err(BotCommandError::new(BotCommandErrorKind::InvalidArgument {
            command: "members.timeout".to_string(),
            arg_name: "duration_seconds".to_string(),
            reason: "Timeout duration cannot exceed 28 days (2419200 seconds)".to_string(),
        }));
    }

    tracing::Span::current().record("guild_id", guild_id.get());
    tracing::Span::current().record("user_id", user_id.get());
    tracing::Span::current().record("duration_seconds", duration_seconds);
    info!(
        guild_id = %guild_id,
        user_id = %user_id,
        duration_seconds,
        "Timing out member"
    );

    let timeout_until = Timestamp::now().unix_timestamp() + duration_seconds as i64;
    let timeout_timestamp = Timestamp::from_unix_timestamp(timeout_until).map_err(|e| {
        error!(error = %e, "Failed to create timeout timestamp");
        BotCommandError::new(BotCommandErrorKind::InvalidArgument {
            command: "members.timeout".to_string(),
            arg_name: "duration_seconds".to_string(),
            reason: format!("Invalid duration: {}", e),
        })
    })?;

    let builder = EditMember::new().disable_communication_until(timeout_timestamp.to_string());

    http.edit_member(guild_id, user_id, &builder, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to timeout member");
            BotCommandError::from_api_error("members.timeout", e)
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

/// Execute: members.remove_timeout
///
/// Remove timeout from a member.
#[instrument(skip(http, args), fields(guild_id, user_id))]
pub(super) async fn remove_timeout(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let user_id_str = args
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("user_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;
    let user_id = parse_user_id(user_id_str)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    tracing::Span::current().record("user_id", user_id.get());
    info!(guild_id = %guild_id, user_id = %user_id, "Removing member timeout");

    let builder = EditMember::new().disable_communication_until_datetime(Timestamp::now());

    guild_id
        .edit_member(http, user_id, builder)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to remove timeout");
            BotCommandError::from_api_error("members.remove_timeout", e)
        })?;

    info!("Successfully removed member timeout");
    Ok(serde_json::json!({
        "guild_id": guild_id.to_string(),
        "user_id": user_id_str,
        "timeout_removed": true,
    }))
}

// Helper functions

#[instrument(skip(arg_name))]
fn missing_arg_error(arg_name: &str) -> BotCommandError {
    BotCommandError::new(BotCommandErrorKind::MissingArgument {
        command: "".to_string(),
        arg_name: arg_name.to_string(),
    })
}

#[instrument(skip(s))]
fn parse_guild_id(s: &str) -> BotCommandResult<GuildId> {
    s.parse::<u64>()
        .map(GuildId::new)
        .map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "".to_string(),
                arg_name: "guild_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })
}

#[instrument(skip(s))]
fn parse_user_id(s: &str) -> BotCommandResult<UserId> {
    s.parse::<u64>()
        .map(UserId::new)
        .map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "".to_string(),
                arg_name: "user_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })
}
