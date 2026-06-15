//! Discord bot client setup and lifecycle management.
//!
//! This module provides the BotticelliBot struct which manages the Discord client
//! connection, event handling, and KV storage integration.

use crate::{BotticelliHandler, DiscordError, DiscordErrorKind, DiscordRepository};
use botticelli_interface::BotStorage;
use serenity::Client;
use std::sync::Arc;
use tracing::{info, instrument};

/// Main Discord bot client for Botticelli.
///
/// Manages the Serenity client connection and integrates with the KV storage
/// backend via DiscordRepository.
///
/// # Example
/// ```no_run
/// use botticelli_social::BotticelliBot;
/// use botticelli_interface::BotStorage;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let token = std::env::var("DISCORD_TOKEN")?;
///     // storage: Arc<dyn BotStorage> provided by caller
///     # let storage: Arc<dyn BotStorage> = unimplemented!();
///     let mut bot = BotticelliBot::new(token, storage).await?;
///     bot.start().await?;
///     Ok(())
/// }
/// ```
pub struct BotticelliBot {
    /// Serenity client instance
    client: Client,
    /// Repository for direct storage access
    repository: Arc<DiscordRepository>,
}

impl BotticelliBot {
    /// Create a new BotticelliBot instance.
    ///
    /// # Arguments
    /// * `token` - Discord bot token from the Discord Developer Portal
    /// * `storage` - Backend storage implementation
    ///
    /// # Errors
    /// Returns an error if:
    /// - The bot token is invalid
    /// - The Serenity client fails to initialize
    #[instrument(skip(token, storage), fields(token_len = token.len()))]
    pub async fn new(
        token: String,
        storage: Arc<dyn BotStorage>,
    ) -> Result<Self, DiscordError> {
        info!("Initializing Botticelli Discord bot");

        let repository = Arc::new(DiscordRepository::new(storage));

        let handler = BotticelliHandler::new(repository.clone());
        let intents = BotticelliHandler::intents();

        info!("Building Serenity client with intents: {:?}", intents);

        let client = Client::builder(&token, intents)
            .event_handler(handler)
            .await
            .map_err(|e| {
                DiscordError::new(DiscordErrorKind::ConnectionFailed(format!(
                    "Failed to build client: {}",
                    e
                )))
            })?;

        info!("Serenity client built successfully");

        Ok(Self { client, repository })
    }

    /// Start the Discord bot.
    ///
    /// This method blocks until the bot is shut down (e.g., via Ctrl+C).
    ///
    /// # Errors
    /// Returns an error if the client fails to start or encounters a fatal error.
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> Result<(), DiscordError> {
        info!("Starting Discord bot");

        self.client.start().await.map_err(|e| {
            DiscordError::new(DiscordErrorKind::ConnectionFailed(format!(
                "Client error: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Get a reference to the repository for direct storage access.
    pub fn repository(&self) -> &Arc<DiscordRepository> {
        &self.repository
    }

    /// Get a reference to the HTTP client for making Discord API requests.
    pub fn http_client(&self) -> Arc<serenity::http::Http> {
        self.client.http.clone()
    }
}
