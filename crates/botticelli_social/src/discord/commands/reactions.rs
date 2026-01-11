//! Reaction commands for Discord messages.
//!
//! This module handles message reaction operations such as adding, removing,
//! listing, and clearing reactions.

use crate::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{ChannelId, Http, MessageId, ReactionType, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

/// Execute: reactions.add
///
/// Add a reaction to a message.
pub(super) async fn add(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;
    let message_id_str = args
        .get("message_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("message_id"))?;
    let emoji_str = args
        .get("emoji")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("emoji"))?;

    let channel_id = parse_channel_id(channel_id_str)?;
    let message_id = parse_message_id(message_id_str)?;

    info!(
        channel_id = %channel_id,
        message_id = %message_id,
        emoji = emoji_str,
        "Adding reaction"
    );

    let reaction = parse_reaction_type(emoji_str)?;

    http.create_reaction(channel_id, message_id, &reaction)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to add reaction");
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
pub(super) async fn remove(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;
    let message_id_str = args
        .get("message_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("message_id"))?;
    let emoji_str = args
        .get("emoji")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("emoji"))?;
    let user_id_str = args
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("user_id"))?;

    let channel_id = parse_channel_id(channel_id_str)?;
    let message_id = parse_message_id(message_id_str)?;
    let user_id = parse_user_id(user_id_str)?;

    info!(
        channel_id = %channel_id,
        message_id = %message_id,
        user_id = %user_id,
        emoji = emoji_str,
        "Removing reaction"
    );

    let reaction = parse_reaction_type(emoji_str)?;

    http.delete_reaction(channel_id, message_id, user_id, &reaction)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to remove reaction");
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

/// Execute: reactions.list
///
/// List users who reacted with a specific emoji.
pub(super) async fn list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;
    let message_id_str = args
        .get("message_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("message_id"))?;
    let emoji_str = args
        .get("emoji")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("emoji"))?;

    let channel_id = parse_channel_id(channel_id_str)?;
    let message_id = parse_message_id(message_id_str)?;
    let reaction = parse_reaction_type(emoji_str)?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(25) as u8;

    info!(
        channel_id = %channel_id,
        message_id = %message_id,
        emoji = emoji_str,
        limit,
        "Listing reactions"
    );

    let users = http
        .get_reaction_users(channel_id, message_id, &reaction, limit, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to list reactions");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "reactions.list".to_string(),
                reason: format!("Failed to list reactions: {}", e),
            })
        })?;

    let user_list: Vec<JsonValue> = users
        .iter()
        .map(|user| {
            serde_json::json!({
                "id": user.id.to_string(),
                "name": user.name,
                "discriminator": user.discriminator,
                "bot": user.bot
            })
        })
        .collect();

    Ok(serde_json::json!({
        "users": user_list,
        "count": user_list.len()
    }))
}

/// Execute: reactions.clear
///
/// Clear all reactions from a message.
pub(super) async fn clear(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;
    let message_id_str = args
        .get("message_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("message_id"))?;

    let channel_id = parse_channel_id(channel_id_str)?;
    let message_id = parse_message_id(message_id_str)?;

    info!(
        channel_id = %channel_id,
        message_id = %message_id,
        "Clearing all reactions"
    );

    http.delete_message_reactions(channel_id, message_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to clear reactions");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "reactions.clear".to_string(),
                reason: format!("Failed to clear reactions: {}", e),
            })
        })?;

    info!("Successfully cleared all reactions");
    Ok(serde_json::json!({ "success": true }))
}

/// Execute: reactions.clear_emoji
///
/// Clear all reactions of a specific emoji from a message.
pub(super) async fn clear_emoji(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;
    let message_id_str = args
        .get("message_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("message_id"))?;
    let emoji_str = args
        .get("emoji")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("emoji"))?;

    let channel_id = parse_channel_id(channel_id_str)?;
    let message_id = parse_message_id(message_id_str)?;
    let reaction = parse_reaction_type(emoji_str)?;

    info!(
        channel_id = %channel_id,
        message_id = %message_id,
        emoji = emoji_str,
        "Clearing emoji reactions"
    );

    http.delete_message_reaction_emoji(channel_id, message_id, &reaction)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to clear emoji reactions");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "reactions.clear_emoji".to_string(),
                reason: format!("Failed to clear emoji reactions: {}", e),
            })
        })?;

    info!("Successfully cleared emoji reactions");
    Ok(serde_json::json!({ "success": true, "emoji": emoji_str }))
}

// Helper functions

fn missing_arg_error(arg_name: &str) -> BotCommandError {
    BotCommandError::new(BotCommandErrorKind::MissingArgument {
        command: "".to_string(),
        arg_name: arg_name.to_string(),
    })
}

fn parse_channel_id(s: &str) -> BotCommandResult<ChannelId> {
    s.parse::<u64>()
        .map(ChannelId::new)
        .map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "".to_string(),
                arg_name: "channel_id".to_string(),
                reason: "Invalid Discord ID format".to_string(),
            })
        })
}

fn parse_message_id(s: &str) -> BotCommandResult<MessageId> {
    s.parse::<u64>()
        .map(MessageId::new)
        .map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "".to_string(),
                arg_name: "message_id".to_string(),
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

fn parse_reaction_type(emoji_str: &str) -> BotCommandResult<ReactionType> {
    // Try to parse as custom emoji or use as Unicode
    if emoji_str.starts_with("custom:") {
        let emoji_id = emoji_str
            .strip_prefix("custom:")
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "".to_string(),
                    arg_name: "emoji".to_string(),
                    reason: "Custom emoji must be in format 'custom:ID'".to_string(),
                })
            })?;
        Ok(ReactionType::Custom {
            animated: false,
            id: emoji_id.into(),
            name: Some("custom".to_string()),
        })
    } else {
        Ok(ReactionType::Unicode(emoji_str.to_string()))
    }
}
