//! Message commands for Discord channels.
//!
//! This module handles message operations such as sending, editing, deleting,
//! listing, and bulk operations on messages.

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{ChannelId, CreateMessage, EditMessage, Http, MessageId};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};

/// Execute: messages.send
///
/// Send a message to a channel.
#[instrument(skip(http, args), fields(channel_id, content_len, tts))]
pub(super) async fn send(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("content"))?;

    let channel_id = parse_channel_id(channel_id_str)?;
    let tts = args.get("tts").and_then(|v| v.as_bool()).unwrap_or(false);

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("content_len", content.len());
    tracing::Span::current().record("tts", tts);
    info!(
        channel_id = %channel_id,
        content_len = content.len(),
        tts,
        "Sending message"
    );

    const MAX_MESSAGE_LENGTH: usize = 2000;

    if content.len() <= MAX_MESSAGE_LENGTH {
        let message = channel_id
            .send_message(http, CreateMessage::new().content(content).tts(tts))
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to send message");
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
        let mut message_ids = Vec::new();
        let chunks: Vec<&str> = content
            .as_bytes()
            .chunks(MAX_MESSAGE_LENGTH)
            .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
            .collect();

        for chunk in chunks {
            let message = channel_id
                .send_message(http, CreateMessage::new().content(chunk).tts(tts))
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to send message chunk");
                    BotCommandError::new(BotCommandErrorKind::ApiError {
                        command: "messages.send".to_string(),
                        reason: format!("Failed to send message chunk: {}", e),
                    })
                })?;
            message_ids.push(message.id.to_string());
        }

        info!(count = message_ids.len(), "Successfully sent multi-part message");
        Ok(serde_json::json!({
            "message_ids": message_ids,
            "channel_id": channel_id.to_string(),
            "parts": message_ids.len(),
        }))
    }
}

/// Execute: messages.get
///
/// Get a specific message.
#[instrument(skip(http, args), fields(channel_id, message_id))]
pub(super) async fn get(
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

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("message_id", message_id.get());
    debug!(channel_id = %channel_id, message_id = %message_id, "Fetching message");

    let message = http
        .get_message(channel_id, message_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to fetch message");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "messages.get".to_string(),
                reason: format!("Failed to fetch message: {}", e),
            })
        })?;

    info!("Successfully retrieved message");
    Ok(serde_json::json!({
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
    }))
}

/// Execute: messages.list
///
/// List messages from a channel (message history).
#[instrument(skip(http, args), fields(channel_id, limit, message_count))]
pub(super) async fn list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;

    let channel_id = parse_channel_id(channel_id_str)?;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|l| l.min(100) as u8)
        .unwrap_or(50);

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("limit", limit);
    debug!(channel_id = %channel_id, limit, "Fetching messages");

    let messages = http
        .get_messages(channel_id, None, Some(limit))
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to fetch messages");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "messages.list".to_string(),
                reason: format!("Failed to fetch messages: {}", e),
            })
        })?;

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

    tracing::Span::current().record("message_count", messages_json.len());
    info!(message_count = messages_json.len(), "Successfully retrieved messages");
    Ok(serde_json::json!(messages_json))
}

/// Execute: messages.edit
///
/// Edit an existing message.
#[instrument(skip(http, args), fields(channel_id, message_id))]
pub(super) async fn edit(
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
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("content"))?;

    let channel_id = parse_channel_id(channel_id_str)?;
    let message_id = parse_message_id(message_id_str)?;

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("message_id", message_id.get());
    info!(channel_id = %channel_id, message_id = %message_id, "Editing message");

    let builder = EditMessage::new().content(content);
    let edited_message = http
        .edit_message(channel_id, message_id, &builder, Vec::new())
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to edit message");
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
#[instrument(skip(http, args), fields(channel_id, message_id))]
pub(super) async fn delete(
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
    let reason = args.get("reason").and_then(|v| v.as_str());

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("message_id", message_id.get());
    info!(channel_id = %channel_id, message_id = %message_id, ?reason, "Deleting message");

    http.delete_message(channel_id, message_id, reason)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to delete message");
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

/// Execute: messages.pin
///
/// Pin a message in a channel.
#[instrument(skip(http, args), fields(channel_id, message_id))]
pub(super) async fn pin(
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

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("message_id", message_id.get());
    info!(channel_id = %channel_id, message_id = %message_id, "Pinning message");

    http.pin_message(channel_id, message_id, None)
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
        "pinned": true,
        "channel_id": channel_id.to_string(),
        "message_id": message_id.to_string(),
    }))
}

/// Execute: messages.unpin
///
/// Unpin a message from a channel.
#[instrument(skip(http, args), fields(channel_id, message_id))]
pub(super) async fn unpin(
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

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("message_id", message_id.get());
    info!(channel_id = %channel_id, message_id = %message_id, "Unpinning message");

    http.unpin_message(channel_id, message_id, None)
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
        "unpinned": true,
        "channel_id": channel_id.to_string(),
        "message_id": message_id.to_string(),
    }))
}

/// Execute: messages.bulk_delete
///
/// Bulk delete messages (up to 100 messages, must be less than 14 days old).
#[instrument(skip(http, args), fields(channel_id, count))]
pub(super) async fn bulk_delete(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;
    let message_ids_array = args
        .get("message_ids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| missing_arg_error("message_ids"))?;

    let channel_id = parse_channel_id(channel_id_str)?;

    let message_ids: Result<Vec<u64>, _> = message_ids_array
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or_else(|| {
                    BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                        command: "messages.bulk_delete".to_string(),
                        arg_name: "message_ids".to_string(),
                        reason: "All message IDs must be strings".to_string(),
                    })
                })
                .and_then(|s| {
                    s.parse::<u64>().map_err(|_| {
                        BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                            command: "messages.bulk_delete".to_string(),
                            arg_name: "message_ids".to_string(),
                            reason: "Invalid Discord ID format".to_string(),
                        })
                    })
                })
        })
        .collect();

    let message_ids = message_ids?;

    if message_ids.len() > 100 {
        return Err(BotCommandError::new(BotCommandErrorKind::InvalidArgument {
            command: "messages.bulk_delete".to_string(),
            arg_name: "message_ids".to_string(),
            reason: "Cannot delete more than 100 messages at once".to_string(),
        }));
    }

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("count", message_ids.len());
    warn!(
        channel_id = %channel_id,
        count = message_ids.len(),
        "Bulk deleting messages"
    );

    let message_ids_json = serde_json::to_value(&message_ids).map_err(|e| {
        error!(error = %e, "Failed to serialize message IDs");
        BotCommandError::new(BotCommandErrorKind::InvalidArgument {
            command: "messages.bulk_delete".to_string(),
            arg_name: "message_ids".to_string(),
            reason: format!("Failed to serialize message IDs: {}", e),
        })
    })?;

    http.delete_messages(channel_id, &message_ids_json, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to bulk delete messages");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "messages.bulk_delete".to_string(),
                reason: format!("Failed to bulk delete messages: {}", e),
            })
        })?;

    info!(count = message_ids.len(), "Successfully bulk deleted messages");
    Ok(serde_json::json!({
        "deleted": message_ids.len(),
        "channel_id": channel_id.to_string(),
    }))
}

/// Execute: messages.clear
///
/// Clear all messages from a channel (bulk delete with fetch).
#[instrument(skip(http, args), fields(channel_id, limit))]
pub(super) async fn clear(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;

    let channel_id = parse_channel_id(channel_id_str)?;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|l| l.min(100) as u8)
        .unwrap_or(100);

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("limit", limit);
    warn!(channel_id = %channel_id, limit, "Clearing messages");

    let messages = http
        .get_messages(channel_id, None, Some(limit))
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to fetch messages for clearing");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "messages.clear".to_string(),
                reason: format!("Failed to fetch messages: {}", e),
            })
        })?;

    let message_ids: Vec<u64> = messages.iter().map(|m| m.id.get()).collect();

    if message_ids.is_empty() {
        info!("No messages to clear");
        return Ok(serde_json::json!({
            "deleted": 0,
            "channel_id": channel_id.to_string(),
        }));
    }

    let message_ids_json = serde_json::to_value(&message_ids).map_err(|e| {
        error!(error = %e, "Failed to serialize message IDs");
        BotCommandError::new(BotCommandErrorKind::InvalidArgument {
            command: "messages.clear".to_string(),
            arg_name: "message_ids".to_string(),
            reason: format!("Failed to serialize message IDs: {}", e),
        })
    })?;

    http.delete_messages(channel_id, &message_ids_json, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to clear messages");
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "messages.clear".to_string(),
                reason: format!("Failed to clear messages: {}", e),
            })
        })?;

    info!(count = message_ids.len(), "Successfully cleared messages");
    Ok(serde_json::json!({
        "deleted": message_ids.len(),
        "channel_id": channel_id.to_string(),
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

#[instrument(skip(s))]
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
