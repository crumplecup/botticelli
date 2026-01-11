//! Discord bot client setup and lifecycle management.
//!
//! This module provides the BotticelliBot struct which manages the Discord client
//! connection, event handling, and database integration.

use crate::{BotticelliHandler, DiscordRepository};
use botticelli_error::{DiscordError, DiscordErrorKind};
use diesel::pg::PgConnection;
use serenity::Client;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::{error, info, instrument};

/// Main Discord bot client for Botticelli.
///
/// Manages the Serenity client connection and integrates with the database
/// via DiscordRepository. Critical errors from event handlers are sent through
/// an error channel for graceful handling.
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
    /// Serenity client instance (wrapped for task safety)
    client: Arc<Mutex<Option<Client>>>,
    /// Database repository for direct database access
    repository: Arc<DiscordRepository>,
    /// Receiver for critical errors from event handlers
    error_rx: mpsc::UnboundedReceiver<DiscordError>,
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

        // Create error channel for critical errors from event handlers
        let (error_tx, error_rx) = mpsc::unbounded_channel();

        // Create event handler
        let handler = BotticelliHandler::new(repository.clone(), error_tx);

        // Get required gateway intents
        let intents = BotticelliHandler::intents();

        info!("Building Serenity client with intents: {:?}", intents);

        // Build the Serenity client
        let client = Client::builder(&token, intents)
            .event_handler(handler)
            .await
            .map_err(DiscordError::from_connection_error)?;

        info!("Serenity client built successfully");

        Ok(Self {
            client: Arc::new(Mutex::new(Some(client))),
            repository,
            error_rx,
        })
    }

    /// Start the Discord bot.
    ///
    /// This method blocks until the bot is shut down (e.g., via Ctrl+C) or
    /// a critical error occurs in an event handler.
    ///
    /// Critical errors (database connection loss, invalid token, etc.) will
    /// cause the bot to shut down gracefully and return the error.
    ///
    /// # Cancellation Safety
    ///
    /// This method spawns the client in a separate task to ensure graceful
    /// shutdown on critical errors. The error channel monitoring does not
    /// cause cancellation of the client future.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The client fails to start
    /// - A critical error occurs in an event handler
    /// - A fatal network/gateway error occurs
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> Result<(), DiscordError> {
        info!("Starting Discord bot");

        // Take client from Option for spawned task (cancellation safety)
        let client_arc = self.client.clone();
        let mut client = client_arc
            .lock()
            .await
            .take()
            .expect("Client already started");

        // Spawn client in separate task to prevent cancellation
        let shard_manager = client.shard_manager.clone();
        let client_handle = tokio::spawn(async move { client.start().await });

        // Monitor error channel
        tokio::select! {
            // Client task completed
            result = client_handle => {
                match result {
                    Ok(Ok(())) => {
                        info!("Discord client shut down normally");
                        return Ok(());
                    }
                    Ok(Err(e)) => {
                        error!(error = %e, "Discord client failed");
                        return Err(DiscordError::from_connection_error(e));
                    }
                    Err(e) => {
                        error!(error = %e, "Client task panicked");
                        return Err(DiscordError::new(
                            DiscordErrorKind::ConnectionFailed(format!("Client task panicked: {}", e))
                        ));
                    }
                }
            }

            // Critical error from event handler
            Some(err) = self.error_rx.recv() => {
                error!(
                    error = %err,
                    "Critical error in event handler, shutting down bot"
                );

                // Graceful shutdown: signal client to stop
                info!("Initiating graceful shutdown");
                shard_manager.shutdown_all().await;

                // Client task continues running, we just return the error
                // (the spawned task will complete in background)
                return Err(err);
            }
        }
    }

    /// Get a reference to the repository for direct database access.
    ///
    /// Useful for querying Discord data outside of event handlers.
    pub fn repository(&self) -> &Arc<DiscordRepository> {
        &self.repository
    }

    /// Get a reference to the HTTP client for making Discord API requests.
    ///
    /// This allows bot commands to share the bot's authentication and HTTP client,
    /// coordinating rate limits and reducing connections.
    ///
    /// # Panics
    ///
    /// Panics if called after start() or if client was already taken.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use botticelli_social::{BotticelliBot, DiscordCommandExecutor};
    ///
    /// let bot = BotticelliBot::new(token, conn).await?;
    /// let executor = DiscordCommandExecutor::with_http_client(bot.http_client().await);
    /// ```
    pub async fn http_client(&self) -> Arc<serenity::http::Http> {
        self.client
            .lock()
            .await
            .as_ref()
            .expect("Client not initialized or already started")
            .http
            .clone()
    }
}
