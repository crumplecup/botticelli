//! Scheduled events commands.

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{
    CreateScheduledEvent, EditScheduledEvent, GuildId, Http, ScheduledEventId, ScheduledEventType,
    Timestamp,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Helper type for event command operations.
#[derive(Debug, Clone, Copy)]
pub(super) struct Events;

impl Events {
    /// List scheduled events in a server.
    ///
    /// Command: `events.list`
    /// Required arguments: `guild_id`
    #[instrument(skip(http, args), fields(guild_id, event_count))]
    pub async fn list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing guild_id argument");
    let guild_id = parse_guild_id("events.list", args)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    info!(guild_id = %guild_id, "Fetching scheduled events from Discord API");

    // Fetch scheduled events
    let events = http
        .get_scheduled_events(guild_id, false)
        .await
        .map_err(|e| {
            error!(guild_id = %guild_id, error = %e, "Failed to fetch events");
            BotCommandError::from_api_error("events.list", e)
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

    /// Get details about a scheduled event.
    ///
    /// Command: `events.get`
    /// Required arguments: `guild_id`, `event_id`
    #[instrument(skip(http, args), fields(guild_id, event_id))]
    pub async fn get(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let event_id_str = args
        .get("event_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("event_id"))?;

    let guild_id = parse_guild_id_str(guild_id_str)?;
    let event_id = parse_event_id(event_id_str)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    tracing::Span::current().record("event_id", event_id.get());
    debug!("Getting scheduled event details");

    let event = http
        .get_scheduled_event(guild_id, event_id, false)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to get event");
            BotCommandError::from_api_error("events.get", e)
        })?;

    debug!(name = %event.name, "Retrieved event");

    Ok(serde_json::json!({
        "id": event.id.to_string(),
        "name": event.name,
        "start_time": event.start_time.to_string(),
        "description": event.description,
    }))
}

    /// Create a scheduled event.
    ///
    /// Command: `events.create`
    /// Required arguments: `guild_id`, `name`, `start_time`
    /// Optional arguments: `description`, `end_time`, `location`
    #[instrument(skip(http, args), fields(guild_id, name))]
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
    let start_time_str = args
        .get("start_time")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("start_time"))?;

    let guild_id = parse_guild_id_str(guild_id_str)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    tracing::Span::current().record("name", name);
    info!(name, "Creating scheduled event");

    let start_time = Timestamp::parse(start_time_str).map_err(|_| {
        BotCommandError::new(BotCommandErrorKind::InvalidArgument {
            command: "events.create".to_string(),
            arg_name: "start_time".to_string(),
            reason: "Invalid ISO 8601 timestamp format".to_string(),
        })
    })?;

    let mut builder = CreateScheduledEvent::new(ScheduledEventType::External, name, start_time);

    if let Some(description) = args.get("description").and_then(|v| v.as_str()) {
        builder = builder.description(description);
    }

    if let Some(end_time_str) = args.get("end_time").and_then(|v| v.as_str()) {
        let end_time = Timestamp::parse(end_time_str).map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "events.create".to_string(),
                arg_name: "end_time".to_string(),
                reason: "Invalid ISO 8601 timestamp format".to_string(),
            })
        })?;
        builder = builder.end_time(end_time);
    }

    if let Some(location) = args.get("location").and_then(|v| v.as_str()) {
        builder = builder.location(location);
    }

    let event = http
        .create_scheduled_event(guild_id, &builder, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to create event");
            BotCommandError::from_api_error("events.create", e)
        })?;

    info!(event_id = %event.id, "Successfully created event");

    Ok(serde_json::json!({
        "event_id": event.id.to_string(),
        "name": event.name,
    }))
}

    /// Edit a scheduled event.
    ///
    /// Command: `events.edit`
    /// Required arguments: `guild_id`, `event_id`
    /// Optional arguments: `name`, `description`, `start_time`, `location`
    #[instrument(skip(http, args), fields(guild_id, event_id))]
    pub async fn edit(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let event_id_str = args
        .get("event_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("event_id"))?;

    let guild_id = parse_guild_id_str(guild_id_str)?;
    let event_id = parse_event_id(event_id_str)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    tracing::Span::current().record("event_id", event_id.get());
    info!("Editing scheduled event");

    let mut builder = EditScheduledEvent::new();

    if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
        builder = builder.name(name);
    }

    if let Some(description) = args.get("description").and_then(|v| v.as_str()) {
        builder = builder.description(description);
    }

    if let Some(start_time_str) = args.get("start_time").and_then(|v| v.as_str()) {
        let start_time = Timestamp::parse(start_time_str).map_err(|_| {
            BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "events.edit".to_string(),
                arg_name: "start_time".to_string(),
                reason: "Invalid ISO 8601 timestamp format".to_string(),
            })
        })?;
        builder = builder.start_time(start_time);
    }

    if let Some(location) = args.get("location").and_then(|v| v.as_str()) {
        builder = builder.location(location);
    }

    http
        .edit_scheduled_event(guild_id, event_id, &builder, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to edit event");
            BotCommandError::from_api_error("events.edit", e)
        })?;

    info!("Successfully edited event");

    Ok(serde_json::json!({ "success": true }))
}

    /// Delete a scheduled event.
    ///
    /// Command: `events.delete`
    /// Required arguments: `guild_id`, `event_id`
    #[instrument(skip(http, args), fields(guild_id, event_id))]
    pub async fn delete(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    let guild_id_str = args
        .get("guild_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("guild_id"))?;
    let event_id_str = args
        .get("event_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_arg_error("event_id"))?;

    let guild_id = parse_guild_id_str(guild_id_str)?;
    let event_id = parse_event_id(event_id_str)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    tracing::Span::current().record("event_id", event_id.get());
    info!("Deleting scheduled event");

    http
        .delete_scheduled_event(guild_id, event_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to delete event");
            BotCommandError::from_api_error("events.delete", e)
        })?;

    info!("Successfully deleted event");

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

#[instrument(skip(command, args))]
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

    parse_guild_id_str(guild_id_str)
}

#[instrument(skip(id_str))]
fn parse_guild_id_str(id_str: &str) -> BotCommandResult<GuildId> {
    let id_u64: u64 = id_str
        .parse()
        .map_err(|e| BotCommandError::from_parse_error("guild_id", e))?;
    Ok(GuildId::new(id_u64))
}

#[instrument(skip(id_str))]
fn parse_event_id(id_str: &str) -> BotCommandResult<ScheduledEventId> {
    let id_u64: u64 = id_str
        .parse()
        .map_err(|e| BotCommandError::from_parse_error("event_id", e))?;
    Ok(ScheduledEventId::new(id_u64))
}
