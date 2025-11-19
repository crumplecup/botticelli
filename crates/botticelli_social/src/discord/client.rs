//! Discord bot client setup and lifecycle management.
//!
//! This module provides the BotticelliBot struct which manages the Discord client
//! connection, event handling, and database integration.

use super::{DiscordError, DiscordErrorKind, DiscordRepository, handler::BotticelliHandler};
use diesel::pg::PgConnection;
use serenity::Client;
use std::sync::Arc;
use tracing::{info, instrument};

/// Main Discord bot client for Botticelli.
///
/// Manages the Serenity client connection and integrates with the database
/// via DiscordRepository.
///
/// # Example
/// ```no_run
/// use botticelli_social::BotticelliBot;
/// use botticelli_database::establish_connection;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let token = std::env::var("DISCORD_TOKEN")?;
///     let conn = establish_connection()?;
///
///     let mut bot = BotticelliBot::new(token, conn).await?;
///     bot.start().await?;
///     Ok(())
/// }
/// ```
pub struct BotticelliBot {
    /// Serenity client instance
    client: Client,
    /// Database repository (kept for potential direct access)
    #[allow(dead_code)]
    repository: Arc<DiscordRepository>,
}

impl BotticelliBot {
    /// Create a new BotticelliBot instance.
    ///
    /// # Arguments
    /// * `token` - Discord bot token from the Discord Developer Portal
    /// * `conn` - PostgreSQL database connection
    ///
    /// # Errors
    /// Returns an error if:
    /// - The bot token is invalid
    /// - The Serenity client fails to initialize
    /// - Database connection fails
    #[instrument(skip(token, conn), fields(token_len = token.len()))]
    pub async fn new(token: String, conn: PgConnection) -> Result<Self, DiscordError> {
        info!("Initializing Botticelli Discord bot");

        // Wrap connection in Arc<Mutex> for async access
        let repository = Arc::new(DiscordRepository::new(conn));

        // Create event handler
        let handler = BotticelliHandler::new(repository.clone());

        // Get required gateway intents
        let intents = BotticelliHandler::intents();

        info!("Building Serenity client with intents: {:?}", intents);

        // Build the Serenity client
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

    /// Get a reference to the repository for direct database access.
    ///
    /// Useful for querying Discord data outside of event handlers.
    #[allow(dead_code)]
    pub fn repository(&self) -> &Arc<DiscordRepository> {
        &self.repository
    }
}
