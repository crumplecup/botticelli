//! Bot server command handler.

use botticelli::GeminiClient;
use botticelli_bot::{BotConfig, BotServer};
use botticelli_database::create_pool;
use botticelli_error::{BackendError, BotticelliResult};
use botticelli_narrative::NarrativeExecutor;
use std::path::PathBuf;

/// Handle the `server` command
pub async fn handle_server_command(
    config_path: Option<PathBuf>,
    _only_bots: Option<String>,
) -> BotticelliResult<()> {
    tracing::info!("Starting bot server");

    // Load bot configuration
    let config_path = config_path.unwrap_or_else(|| PathBuf::from("bot_server.toml"));
    let bot_config = BotConfig::from_file(&config_path)?;

    // Create Gemini client using config from botticelli.toml
    let client = GeminiClient::new_with_tier(None)?;

    // Create database connection pool
    let pool = create_pool()?;

    // Create narrative executor
    let executor = NarrativeExecutor::new(client);

    // Create and start server
    let server = BotServer::new(bot_config, executor, pool);

    tracing::info!("Bot server starting. Press Ctrl+C to stop.");

    server
        .start()
        .await
        .map_err(|e| BackendError::new(e.to_string()))?;

    Ok(())
}
