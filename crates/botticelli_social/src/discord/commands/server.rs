//! Server-related Discord commands.

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{GuildId, Http};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Get server statistics.
///
/// Command: `server.get_stats`
/// Required arguments: `guild_id`
pub(super) async fn get_stats(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing guild_id argument");
    let guild_id = parse_guild_id("server.get_stats", args)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    info!(guild_id = %guild_id, "Fetching guild stats from Discord API");

    // Fetch guild data
    let guild = http.get_guild(guild_id).await.map_err(|e| {
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

    info!(member_count, "Successfully retrieved guild stats");

    Ok(stats)
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
