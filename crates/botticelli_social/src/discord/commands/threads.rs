//! Thread commands for Discord channels.
//!
//! This module handles thread operations such as creating, editing, deleting,
//! and managing thread membership.

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{ChannelId, ChannelType, CreateThread, EditThread, GuildId, Http, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Threads command namespace.
///
/// This zero-sized type provides a clean namespace for thread-related
/// Discord commands without polluting the module with free functions.
#[derive(Debug, Clone, Copy)]
pub(super) struct Threads;

impl Threads {
    /// Execute: threads.create
    ///
    /// Create a new thread in a channel.
    #[instrument(skip(http, args), fields(channel_id, name))]
    pub async fn create(
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

    let channel_id = parse_channel_id(channel_id_str)?;

    tracing::Span::current().record("channel_id", channel_id.get());
    tracing::Span::current().record("name", name);
    info!(channel_id = %channel_id, name, "Creating thread");

    let builder = CreateThread::new(name.to_string()).kind(ChannelType::PublicThread);

    let thread = http
        .create_thread(channel_id, &builder, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to create thread");
            BotCommandError::from_api_error("threads.create", e)
        })?;

    info!(thread_id = %thread.id, "Successfully created thread");
    Ok(serde_json::json!({
        "thread_id": thread.id.to_string(),
        "name": thread.name,
        "type": format!("{:?}", thread.kind)
    }))
}

    /// Execute: threads.list
    ///
    /// List active threads in a guild.
    #[instrument(skip(http, args), fields(guild_id, count))]
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
    debug!(guild_id = %guild_id, "Listing threads");

    let threads = http
        .get_guild_active_threads(guild_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to list threads");
            BotCommandError::from_api_error("threads.list", e)
        })?;

    let thread_list: Vec<JsonValue> = threads
        .threads
        .iter()
        .map(|thread| {
            serde_json::json!({
                "id": thread.id.to_string(),
                "name": thread.name,
                "type": format!("{:?}", thread.kind),
                "parent_id": thread.parent_id.map(|id| id.to_string())
            })
        })
        .collect();

    tracing::Span::current().record("count", thread_list.len());
    info!(count = thread_list.len(), "Successfully listed threads");
    Ok(serde_json::json!({
        "threads": thread_list,
        "count": thread_list.len()
    }))
}

    /// Execute: threads.get
    ///
    /// Get specific thread details.
    #[instrument(skip(http, args), fields(thread_id))]
    pub async fn get(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let thread_id_str = args
        .get("thread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("thread_id"))?;

    let thread_id = parse_channel_id(thread_id_str)?;

    tracing::Span::current().record("thread_id", thread_id.get());
    debug!(thread_id = %thread_id, "Fetching thread");

    let thread = http.get_channel(thread_id).await.map_err(|e| {
        error!(error = %e, "Failed to get thread");
        BotCommandError::from_api_error("threads.get", e)
    })?;

    let guild_channel = thread.guild().ok_or_else(|| {
        error!("Channel is not a guild channel");
        BotCommandError::new(BotCommandErrorKind::ApiError {
            command: "threads.get".to_string(),
            reason: "Channel is not a guild channel".to_string(),
        })
    })?;

    info!(thread_id = %thread_id, "Successfully retrieved thread");
    Ok(serde_json::json!({
        "id": guild_channel.id.to_string(),
        "name": guild_channel.name,
        "type": format!("{:?}", guild_channel.kind),
        "parent_id": guild_channel.parent_id.map(|id| id.to_string())
    }))
}

    /// Execute: threads.edit
    ///
    /// Edit thread properties.
    #[instrument(skip(http, args), fields(thread_id))]
    pub async fn edit(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let thread_id_str = args
        .get("thread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("thread_id"))?;

    let thread_id = parse_channel_id(thread_id_str)?;

    tracing::Span::current().record("thread_id", thread_id.get());
    info!(thread_id = %thread_id, "Editing thread");

    let mut builder = EditThread::new();

    if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
        builder = builder.name(name);
    }
    if let Some(archived) = args.get("archived").and_then(|v| v.as_bool()) {
        builder = builder.archived(archived);
    }
    if let Some(locked) = args.get("locked").and_then(|v| v.as_bool()) {
        builder = builder.locked(locked);
    }

    http.edit_thread(thread_id, &builder, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to edit thread");
            BotCommandError::from_api_error("threads.edit", e)
        })?;

    info!(thread_id = %thread_id, "Successfully edited thread");
    Ok(serde_json::json!({ "success": true }))
}

    /// Execute: threads.delete
    ///
    /// Delete a thread.
    #[instrument(skip(http, args), fields(thread_id))]
    pub async fn delete(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let thread_id_str = args
        .get("thread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("thread_id"))?;

    let thread_id = parse_channel_id(thread_id_str)?;

    tracing::Span::current().record("thread_id", thread_id.get());
    info!(thread_id = %thread_id, "Deleting thread");

    http.delete_channel(thread_id, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to delete thread");
            BotCommandError::from_api_error("threads.delete", e)
        })?;

    info!(thread_id = %thread_id, "Successfully deleted thread");
    Ok(serde_json::json!({ "success": true }))
}

    /// Execute: threads.join
    ///
    /// Join a thread.
    #[instrument(skip(http, args), fields(thread_id))]
    pub async fn join(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let thread_id_str = args
        .get("thread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("thread_id"))?;

    let thread_id = parse_channel_id(thread_id_str)?;

    tracing::Span::current().record("thread_id", thread_id.get());
    info!(thread_id = %thread_id, "Joining thread");

    http.join_thread_channel(thread_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to join thread");
            BotCommandError::from_api_error("threads.join", e)
        })?;

    info!(thread_id = %thread_id, "Successfully joined thread");
    Ok(serde_json::json!({ "success": true }))
}

    /// Execute: threads.leave
    ///
    /// Leave a thread.
    #[instrument(skip(http, args), fields(thread_id))]
    pub async fn leave(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let thread_id_str = args
        .get("thread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("thread_id"))?;

    let thread_id = parse_channel_id(thread_id_str)?;

    tracing::Span::current().record("thread_id", thread_id.get());
    info!(thread_id = %thread_id, "Leaving thread");

    http.leave_thread_channel(thread_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to leave thread");
            BotCommandError::from_api_error("threads.leave", e)
        })?;

    info!(thread_id = %thread_id, "Successfully left thread");
    Ok(serde_json::json!({ "success": true }))
}

    /// Execute: threads.add_member
    ///
    /// Add a member to a thread.
    #[instrument(skip(http, args), fields(thread_id, user_id))]
    pub async fn add_member(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let thread_id_str = args
        .get("thread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("thread_id"))?;
    let user_id_str = args
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("user_id"))?;

    let thread_id = parse_channel_id(thread_id_str)?;
    let user_id = parse_user_id(user_id_str)?;

    tracing::Span::current().record("thread_id", thread_id.get());
    tracing::Span::current().record("user_id", user_id.get());
    info!(thread_id = %thread_id, user_id = %user_id, "Adding member to thread");

    http.add_thread_channel_member(thread_id, user_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to add member to thread");
            BotCommandError::from_api_error("threads.add_member", e)
        })?;

    info!("Successfully added member to thread");
    Ok(serde_json::json!({ "success": true }))
}

    /// Execute: threads.remove_member
    ///
    /// Remove a member from a thread.
    #[instrument(skip(http, args), fields(thread_id, user_id))]
    pub async fn remove_member(
        http: &Arc<Http>,
        args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let thread_id_str = args
        .get("thread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("thread_id"))?;
    let user_id_str = args
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("user_id"))?;

    let thread_id = parse_channel_id(thread_id_str)?;
    let user_id = parse_user_id(user_id_str)?;

    tracing::Span::current().record("thread_id", thread_id.get());
    tracing::Span::current().record("user_id", user_id.get());
    info!(thread_id = %thread_id, user_id = %user_id, "Removing member from thread");

    http.remove_thread_channel_member(thread_id, user_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to remove member from thread");
            BotCommandError::from_api_error("threads.remove_member", e)
        })?;

    info!("Successfully removed member from thread");
    Ok(serde_json::json!({ "success": true }))
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
fn parse_guild_id(s: &str) -> BotCommandResult<GuildId> {
    s.parse::<u64>()
        .map(GuildId::new)
        .map_err(|e| BotCommandError::from_parse_error("guild_id", e))
}

#[instrument(skip(s))]
fn parse_channel_id(s: &str) -> BotCommandResult<ChannelId> {
    s.parse::<u64>()
        .map(ChannelId::new)
        .map_err(|e| BotCommandError::from_parse_error("thread_id", e))
}

#[instrument(skip(s))]
fn parse_user_id(s: &str) -> BotCommandResult<UserId> {
    s.parse::<u64>()
        .map(UserId::new)
        .map_err(|e| BotCommandError::from_parse_error("user_id", e))
}
