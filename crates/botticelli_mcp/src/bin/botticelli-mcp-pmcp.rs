//! Botticelli MCP server binary using pmcp SDK.

use anyhow::Result;
use botticelli_mcp::run_pmcp_server;
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    let _ = dotenvy::dotenv();

    // Initialize tracing to file (not stdout - that's for JSON-RPC)
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("botticelli-mcp.log")?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::sync::Arc::new(log_file))
        .init();

    tracing::info!("Starting Botticelli MCP server (PMCP implementation)");

    // Run the pmcp server
    run_pmcp_server(
        #[cfg(feature = "database")]
        None, // TODO: Load database configuration
    )
    .await?;

    Ok(())
}
