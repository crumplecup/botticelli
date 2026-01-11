//! Miscellaneous Discord commands (webhooks, stickers, emojis, invites, integrations, voice regions).

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use serde_json::Value as JsonValue;
use serenity::all::{GuildId, Http};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Helper type for miscellaneous command operations.
#[derive(Debug, Clone, Copy)]
pub(super) struct Misc;

impl Misc {
    /// List custom emojis in a server.
    ///
    /// Command: `emojis.list`
    /// Required arguments: `guild_id`
    #[instrument(skip(http, args), fields(guild_id, emoji_count))]
    pub async fn emojis_list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing guild_id argument");
    let guild_id = parse_guild_id("emojis.list", args)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    info!(guild_id = %guild_id, "Fetching emojis from Discord API");

    // Fetch emojis
    let emojis = http.get_emojis(guild_id).await.map_err(|e| {
        error!(guild_id = %guild_id, error = %e, "Failed to fetch emojis");
        BotCommandError::from_api_error("emojis.list", e)
    })?;

    let emoji_count = emojis.len();
    tracing::Span::current().record("emoji_count", emoji_count);

    let emojis_json: Vec<JsonValue> = emojis
        .into_iter()
        .map(|emoji| {
            serde_json::json!({
                "id": emoji.id.to_string(),
                "name": emoji.name,
                "animated": emoji.animated,
                "managed": emoji.managed,
                "require_colons": emoji.require_colons,
                "available": emoji.available,
            })
        })
        .collect();

    info!(emoji_count, "Successfully retrieved emojis");

    Ok(serde_json::json!(emojis_json))
}

    /// List custom stickers in a server.
    ///
    /// Command: `stickers.list`
    /// Required arguments: `guild_id`
    #[instrument(skip(http, args), fields(guild_id, sticker_count))]
    pub async fn stickers_list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing guild_id argument");
    let guild_id = parse_guild_id("stickers.list", args)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    info!(guild_id = %guild_id, "Fetching stickers from Discord API");

    // Fetch stickers
    let stickers = http.get_guild_stickers(guild_id).await.map_err(|e| {
        error!(guild_id = %guild_id, error = %e, "Failed to fetch stickers");
        BotCommandError::from_api_error("stickers.list", e)
    })?;

    let sticker_count = stickers.len();
    tracing::Span::current().record("sticker_count", sticker_count);

    let stickers_json: Vec<JsonValue> = stickers
        .into_iter()
        .map(|sticker| {
            serde_json::json!({
                "id": sticker.id.to_string(),
                "name": sticker.name,
                "description": sticker.description,
                "tags": sticker.tags,
                "format_type": format!("{:?}", sticker.format_type),
                "available": sticker.available,
            })
        })
        .collect();

    info!(sticker_count, "Successfully retrieved stickers");

    Ok(serde_json::json!(stickers_json))
}

    /// List active invites in a server.
    ///
    /// Command: `invites.list`
    /// Required arguments: `guild_id`
    #[instrument(skip(http, args), fields(guild_id, invite_count))]
    pub async fn invites_list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing guild_id argument");
    let guild_id = parse_guild_id("invites.list", args)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    info!(guild_id = %guild_id, "Fetching invites from Discord API");

    // Fetch invites
    let invites = http.get_guild_invites(guild_id).await.map_err(|e| {
        error!(guild_id = %guild_id, error = %e, "Failed to fetch invites");
        BotCommandError::from_api_error("invites.list", e)
    })?;

    let invite_count = invites.len();
    tracing::Span::current().record("invite_count", invite_count);

    let invites_json: Vec<JsonValue> = invites
        .into_iter()
        .map(|invite| {
            serde_json::json!({
                "code": invite.code,
                "channel_id": invite.channel.id.to_string(),
                "inviter": invite.inviter.as_ref().map(|u| serde_json::json!({
                    "id": u.id.to_string(),
                    "name": u.name.clone(),
                })),
                "uses": invite.uses,
                "max_uses": invite.max_uses,
                "max_age": invite.max_age,
                "temporary": invite.temporary,
                "created_at": invite.created_at.to_string(),
            })
        })
        .collect();

    info!(invite_count, "Successfully retrieved invites");

    Ok(serde_json::json!(invites_json))
}

    /// List webhooks in a server.
    ///
    /// Command: `webhooks.list`
    /// Required arguments: `guild_id`
    #[instrument(skip(http, args), fields(guild_id, webhook_count))]
    pub async fn webhooks_list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing guild_id argument");
    let guild_id = parse_guild_id("webhooks.list", args)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    info!(guild_id = %guild_id, "Fetching webhooks from Discord API");

    // Fetch webhooks
    let webhooks = http.get_guild_webhooks(guild_id).await.map_err(|e| {
        error!(guild_id = %guild_id, error = %e, "Failed to fetch webhooks");
        BotCommandError::from_api_error("webhooks.list", e)
    })?;

    let webhook_count = webhooks.len();
    tracing::Span::current().record("webhook_count", webhook_count);

    let webhooks_json: Vec<JsonValue> = webhooks
        .into_iter()
        .map(|webhook| {
            serde_json::json!({
                "id": webhook.id.to_string(),
                "name": webhook.name,
                "channel_id": webhook.channel_id.map(|id| id.to_string()),
                "avatar": webhook.avatar,
                "guild_id": webhook.guild_id.map(|id| id.to_string()),
            })
        })
        .collect();

    info!(webhook_count, "Successfully retrieved webhooks");

    Ok(serde_json::json!(webhooks_json))
}

    /// List integrations in a server.
    ///
    /// Command: `integrations.list`
    /// Required arguments: `guild_id`
    #[instrument(skip(http, args), fields(guild_id, integration_count))]
    pub async fn integrations_list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing guild_id argument");
    let guild_id = parse_guild_id("integrations.list", args)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    info!(guild_id = %guild_id, "Fetching integrations from Discord API");

    // Fetch integrations
    let integrations = http
        .get_guild_integrations(guild_id)
        .await
        .map_err(|e| {
            error!(guild_id = %guild_id, error = %e, "Failed to fetch integrations");
            BotCommandError::from_api_error("integrations.list", e)
        })?;

    let integration_count = integrations.len();
    tracing::Span::current().record("integration_count", integration_count);

    let integrations_json: Vec<JsonValue> = integrations
        .into_iter()
        .map(|integration| {
            serde_json::json!({
                "id": integration.id.to_string(),
                "name": integration.name,
                "type": integration.kind,
                "enabled": integration.enabled,
                "syncing": integration.syncing,
                "account": serde_json::json!({
                    "id": integration.account.id,
                    "name": integration.account.name,
                }),
            })
        })
        .collect();

    info!(integration_count, "Successfully retrieved integrations");

    Ok(serde_json::json!(integrations_json))
}

    /// List available voice regions for a server.
    ///
    /// Command: `voice_regions.list`
    /// Required arguments: `guild_id`
    #[instrument(skip(http, args), fields(guild_id, region_count))]
    pub async fn voice_regions_list(
    http: &Arc<Http>,
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<JsonValue> {
    debug!("Parsing guild_id argument");
    let guild_id = parse_guild_id("voice_regions.list", args)?;

    tracing::Span::current().record("guild_id", guild_id.get());
    info!(guild_id = %guild_id, "Fetching voice regions from Discord API");

    // Fetch voice regions
    let regions = http.get_guild_regions(guild_id).await.map_err(|e| {
        error!(guild_id = %guild_id, error = %e, "Failed to fetch voice regions");
        BotCommandError::from_api_error("voice_regions.list", e)
    })?;

    let region_count = regions.len();
    tracing::Span::current().record("region_count", region_count);

    let regions_json: Vec<JsonValue> = regions
        .into_iter()
        .map(|region| {
            serde_json::json!({
                "id": region.id,
                "name": region.name,
                "optimal": region.optimal,
                "deprecated": region.deprecated,
                "custom": region.custom,
            })
        })
        .collect();

    info!(region_count, "Successfully retrieved voice regions");

    Ok(serde_json::json!(regions_json))
}
}

/// Parse guild_id from command arguments.
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

    let guild_id = guild_id_str.parse::<u64>().map_err(|e| {
        BotCommandError::new(BotCommandErrorKind::InvalidArgument {
            command: command.to_string(),
            arg_name: "guild_id".to_string(),
            reason: format!("Invalid guild_id format: {}", e),
        })
    })?;

    Ok(GuildId::new(guild_id))
}
