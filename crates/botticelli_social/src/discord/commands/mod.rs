//! Discord bot command executor (modular).
//!
//! This module implements the BotCommandExecutor trait for Discord,
//! routing commands to domain-specific submodules.

mod misc;
mod server;

use crate::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use async_trait::async_trait;
use botticelli_interface::BotCommandExecutor;
use serde_json::Value as JsonValue;
use serenity::all::Http;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// Discord command executor.
#[derive(Debug, Clone)]
pub struct DiscordCommandExecutor {
    http: Arc<Http>,
}

impl DiscordCommandExecutor {
    /// Create new Discord command executor with HTTP client.
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

#[async_trait]
impl BotCommandExecutor for DiscordCommandExecutor {
    type Error = BotCommandError;

    fn platform(&self) -> &str {
        "discord"
    }

    #[instrument(
        skip(self, args),
        fields(
            platform = "discord",
            command,
            arg_count = args.len(),
            result_size,
            duration_ms
        )
    )]
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        info!("Executing Discord bot command");

        let start = std::time::Instant::now();

        let result = match command {
            // Server commands
            "server.get_stats" => server::get_stats(&self.http, args).await?,

            // Misc commands
            "emojis.list" => misc::emojis_list(&self.http, args).await?,
            "stickers.list" => misc::stickers_list(&self.http, args).await?,
            "invites.list" => misc::invites_list(&self.http, args).await?,
            "webhooks.list" => misc::webhooks_list(&self.http, args).await?,
            "integrations.list" => misc::integrations_list(&self.http, args).await?,
            "voice_regions.list" => misc::voice_regions_list(&self.http, args).await?,

            // TODO: Add other commands as we migrate them
            
            _ => {
                error!(
                    command,
                    "Command not found or not yet migrated"
                );
                return Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
                    command.to_string(),
                )));
            }
        };

        let duration_ms = start.elapsed().as_millis();
        let result_size = serde_json::to_string(&result).map(|s| s.len()).unwrap_or(0);

        tracing::Span::current().record("duration_ms", duration_ms);
        tracing::Span::current().record("result_size", result_size);
        info!(
            duration_ms,
            result_size, "Discord command executed successfully"
        );

        Ok(result)
    }

    fn supports_command(&self, command: &str) -> bool {
        matches!(
            command,
            // Server
            "server.get_stats"
            // Misc
            | "emojis.list"
            | "stickers.list"
            | "invites.list"
            | "webhooks.list"
            | "integrations.list"
            | "voice_regions.list"
            // TODO: Add other commands as we migrate them
        )
    }

    fn supported_commands(&self) -> Vec<String> {
        vec![
            // Server
            "server.get_stats".to_string(),
            // Misc
            "emojis.list".to_string(),
            "stickers.list".to_string(),
            "invites.list".to_string(),
            "webhooks.list".to_string(),
            "integrations.list".to_string(),
            "voice_regions.list".to_string(),
            // TODO: Add other commands as we migrate them
        ]
    }

    fn command_help(&self, command: &str) -> Option<String> {
        match command {
            "server.get_stats" => Some(
                "Get server statistics (member count, channels, etc.)\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "emojis.list" => Some("List custom emojis\nRequired arguments: guild_id".to_string()),
            "stickers.list" => Some("List custom stickers\nRequired arguments: guild_id".to_string()),
            "invites.list" => Some("List active invites\nRequired arguments: guild_id".to_string()),
            "webhooks.list" => Some("List webhooks\nRequired arguments: guild_id".to_string()),
            "integrations.list" => Some("List integrations\nRequired arguments: guild_id".to_string()),
            "voice_regions.list" => Some("List voice regions\nRequired arguments: guild_id".to_string()),
            // TODO: Add other commands as we migrate them
            _ => None,
        }
    }

    // Trait methods - stub implementations for now
    async fn messages_bulk_delete(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "messages_bulk_delete - not yet migrated".to_string(),
        )))
    }

    async fn threads_create(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "threads_create - not yet migrated".to_string(),
        )))
    }

    async fn threads_list(&self, _args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "threads_list - not yet migrated".to_string(),
        )))
    }

    async fn threads_get(&self, _args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "threads_get - not yet migrated".to_string(),
        )))
    }

    async fn threads_edit(&self, _args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "threads_edit - not yet migrated".to_string(),
        )))
    }

    async fn threads_delete(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "threads_delete - not yet migrated".to_string(),
        )))
    }

    async fn threads_join(&self, _args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "threads_join - not yet migrated".to_string(),
        )))
    }

    async fn threads_leave(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "threads_leave - not yet migrated".to_string(),
        )))
    }

    async fn threads_add_member(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "threads_add_member - not yet migrated".to_string(),
        )))
    }

    async fn threads_remove_member(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "threads_remove_member - not yet migrated".to_string(),
        )))
    }

    async fn reactions_list(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "reactions_list - not yet migrated".to_string(),
        )))
    }

    async fn reactions_clear(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "reactions_clear - not yet migrated".to_string(),
        )))
    }

    async fn reactions_clear_emoji(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "reactions_clear_emoji - not yet migrated".to_string(),
        )))
    }
}
