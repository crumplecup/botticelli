//! Role commands for Discord guilds.
//!
//! This module handles role management operations such as listing, creating,
//! editing, deleting roles, and assigning/removing roles from members.

use crate::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{EditRole, GuildId, Http, RoleId, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Execute: roles.list
///
/// List all roles in a guild.
pub(super) async fn list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;

    debug!(guild_id = %guild_id, "Fetching roles");

    let roles = http.get_guild_roles(guild_id).await.map_err(|e| {
        error!(error = %e, "Failed to fetch roles");
        BotCommandError::new(BotCommandErrorKind::ApiError {
            command: "roles.list".to_string(),
            reason: format!("Failed to fetch roles: {}", e),
        })
    })?;

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

    info!(role_count = roles_json.len(), "Successfully retrieved roles");
    Ok(serde_json::json!(roles_json))
}

/// Execute: roles.get
///
/// Get specific role details.
pub(super) async fn get(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let role_id_str = args
        .get("role_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("role_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;
    let role_id = parse_role_id(role_id_str)?;

    debug!(guild_id = %guild_id, role_id = %role_id, "Fetching role");

    let roles = http.get_guild_roles(guild_id).await.map_err(|e| {
        error!(error = %e, "Failed to fetch roles");
        BotCommandError::new(BotCommandErrorKind::ApiError {
            command: "roles.get".to_string(),
            reason: format!("Failed to fetch roles: {}", e),
        })
    })?;

    let role = roles.into_iter().find(|r| r.id == role_id).ok_or_else(|| {
        error!(guild_id = %guild_id, role_id = %role_id, "Role not found");
        BotCommandError::new(BotCommandErrorKind::ResourceNotFound {
            command: "roles.get".to_string(),
            resource_type: "role".to_string(),
        })
    })?;

    info!(role_id = %role_id, "Successfully retrieved role details");
    Ok(serde_json::json!({
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
    }))
}

/// Execute: roles.create
///
/// Create a new role in the guild.
pub(super) async fn create(
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

    let color = args.get("color").and_then(|v| v.as_u64()).map(|c| c as u32);
    let hoist = args.get("hoist").and_then(|v| v.as_bool()).unwrap_or(false);
    let mentionable = args
        .get("mentionable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    info!(
        guild_id = %guild_id,
        name,
        color,
        hoist,
        mentionable,
        "Creating role"
    );

    let mut builder = EditRole::new()
        .name(name)
        .hoist(hoist)
        .mentionable(mentionable);

    if let Some(c) = color {
        builder = builder.colour(c);
    }

    let role = http
        .create_role(guild_id, &builder, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to create role");
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

/// Execute: roles.edit
///
/// Edit role properties.
pub(super) async fn edit(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let role_id_str = args
        .get("role_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("role_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;
    let role_id = parse_role_id(role_id_str)?;

    info!(guild_id = %guild_id, role_id = %role_id, "Editing role");

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

    let role = http
        .edit_role(guild_id, role_id, &builder, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to edit role");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "roles.edit".to_string(),
                reason: format!("Failed to edit role: {}", e),
            })
        })?;

    info!(role_id = %role_id, "Successfully edited role");
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
pub(super) async fn delete(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let role_id_str = args
        .get("role_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("role_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;
    let role_id = parse_role_id(role_id_str)?;

    info!(guild_id = %guild_id, role_id = %role_id, "Deleting role");

    http.delete_role(guild_id, role_id, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to delete role");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "roles.delete".to_string(),
                reason: format!("Failed to delete role: {}", e),
            })
        })?;

    info!(role_id = %role_id, "Successfully deleted role");
    Ok(serde_json::json!({
        "deleted": true,
        "guild_id": guild_id.to_string(),
        "role_id": role_id.to_string(),
    }))
}

/// Execute: roles.assign
///
/// Assign a role to a member.
pub(super) async fn assign(
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
    let role_id_str = args
        .get("role_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("role_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;
    let user_id = parse_user_id(user_id_str)?;
    let role_id = parse_role_id(role_id_str)?;

    info!(
        guild_id = %guild_id,
        user_id = %user_id,
        role_id = %role_id,
        "Assigning role to member"
    );

    http.add_member_role(guild_id, user_id, role_id, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to assign role");
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
pub(super) async fn remove(
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
    let role_id_str = args
        .get("role_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("role_id"))?;

    let guild_id = parse_guild_id(guild_id_str)?;
    let user_id = parse_user_id(user_id_str)?;
    let role_id = parse_role_id(role_id_str)?;

    info!(
        guild_id = %guild_id,
        user_id = %user_id,
        role_id = %role_id,
        "Removing role from member"
    );

    http.remove_member_role(guild_id, user_id, role_id, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to remove role");
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

// Helper functions

fn missing_arg_error(arg_name: &str) -> BotCommandError {
    BotCommandError::new(BotCommandErrorKind::MissingArgument {
        command: "".to_string(),
        arg_name: arg_name.to_string(),
    })
}

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

fn parse_role_id(s: &str) -> BotCommandResult<RoleId> {
    s.parse::<u64>()
        .map(RoleId::new)
        .map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "".to_string(),
                arg_name: "role_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })
}

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
