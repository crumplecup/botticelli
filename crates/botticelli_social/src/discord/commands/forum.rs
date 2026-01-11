//! Forum post commands for Discord.
//!
//! This module handles forum-related operations such as creating posts,
//! listing posts, and retrieving post details.

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{
    AutoArchiveDuration, Channel, ChannelId, CreateForumPost, CreateMessage, Http,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Execute: forum.create_post
///
/// Create a new forum post (thread in a forum channel).
#[instrument(skip(http, args), fields(channel_id, name))]
pub(super) async fn create_post(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("name"))?;
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("content"))?;

    let channel_id = parse_channel_id(channel_id_str)?;

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("name", name);
    info!(name, "Creating forum post");

    let mut builder = CreateForumPost::new(name, CreateMessage::new().content(content));

    if let Some(duration) = args.get("auto_archive_duration").and_then(|v| v.as_u64()) {
        let auto_archive = match duration {
            60 => AutoArchiveDuration::OneHour,
            1440 => AutoArchiveDuration::OneDay,
            4320 => AutoArchiveDuration::ThreeDays,
            10080 => AutoArchiveDuration::OneWeek,
            _ => AutoArchiveDuration::OneHour, // Default
        };
        builder = builder.auto_archive_duration(auto_archive);
    }

    let thread = channel_id
        .create_forum_post(http, builder)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to create forum post");
            BotCommandError::from_api_error("forum.create_post", e)
        })?;

    info!(thread_id = %thread.id, "Successfully created forum post");

    Ok(serde_json::json!({
        "thread_id": thread.id.to_string(),
        "name": thread.name,
    }))
}

/// Execute: forum.list_posts
///
/// List forum posts (active threads in a forum channel).
#[instrument(skip(_http, args), fields(channel_id))]
pub(super) async fn list_posts(
    _http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let channel_id_str = args
        .get("channel_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("channel_id"))?;

    let _channel_id = parse_channel_id(channel_id_str)?;

    tracing::Span::current().record("channel_id", channel_id_str);
    debug!("Listing forum posts");

    // TODO: Implement forum post listing
    // The Serenity API doesn't have a direct method for this
    Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
        "forum.list_posts not yet implemented".to_string(),
    )))
}

/// Execute: forum.get_post
///
/// Get details about a specific forum post.
#[instrument(skip(http, args), fields(thread_id))]
pub(super) async fn get_post(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let thread_id_str = args
        .get("thread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("thread_id"))?;

    let thread_id = parse_channel_id(thread_id_str)?;

    tracing::Span::current().record("thread_id", thread_id.get());
    debug!("Getting forum post details");

    let channel = http.get_channel(thread_id).await.map_err(|e| {
        error!(error = %e, "Failed to get forum post");
        BotCommandError::from_api_error("forum.get_post", e)
    })?;

    match channel {
        Channel::Guild(guild_channel) => {
            debug!(name = %guild_channel.name, "Retrieved forum post");
            Ok(serde_json::json!({
                "id": guild_channel.id.to_string(),
                "name": guild_channel.name,
                "message_count": guild_channel.message_count.unwrap_or(0),
            }))
        }
        _ => Err(BotCommandError::new(BotCommandErrorKind::InvalidArgument {
            command: "forum.get_post".to_string(),
            arg_name: "thread_id".to_string(),
            reason: "Not a forum post".to_string(),
        })),
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
