//! Bot server command handler.

use botticelli_error::BotticelliResult;
use botticelli_server::BotServer;
use std::path::PathBuf;

/// Handle the `server` command
pub async fn handle_server_command(
    _config_path: Option<PathBuf>,
    _only_bots: Option<String>,
) -> BotticelliResult<()> {
    tracing::info!("Starting bot server");

    // Create and run the server
    let server = BotServer::new().await?;
    server.run().await?;

    Ok(())
}
