//! Bot server command handler.

use botticelli_bot::{BotServerConfig, run_bot_server};
use botticelli_error::BotticelliResult;
use std::path::PathBuf;

/// Handle the `server` command
pub async fn handle_server_command(
    config_path: Option<PathBuf>,
    only_bots: Option<String>,
) -> BotticelliResult<()> {
    // Load configuration
    let config = if let Some(path) = config_path {
        BotServerConfig::from_file(&path)?
    } else {
        BotServerConfig::from_default_locations()?
    };

    // Filter bots if --only flag is provided
    let config = if let Some(only) = only_bots {
        let enabled_bots: Vec<&str> = only.split(',').map(|s| s.trim()).collect();
        config.with_only_bots(&enabled_bots)
    } else {
        config
    };

    // Run the server
    run_bot_server(config).await
}
