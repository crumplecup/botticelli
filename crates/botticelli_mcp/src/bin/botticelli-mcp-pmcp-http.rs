//! Botticelli MCP HTTP server using pmcp SDK.

use anyhow::Result;
use tracing_subscriber::{self, EnvFilter};

#[cfg(feature = "streamable-http")]
use botticelli_mcp::run_pmcp_http_server;

#[cfg(not(feature = "streamable-http"))]
use botticelli_mcp::run_pmcp_server;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    #[cfg(feature = "streamable-http")]
    {
        // Read host and port from environment or use defaults
        let host = std::env::var("MCP_HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("MCP_HTTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        tracing::info!("Starting Botticelli MCP HTTP server (PMCP implementation)");
        run_pmcp_http_server(&host, port).await?;
    }

    #[cfg(not(feature = "streamable-http"))]
    {
        tracing::warn!("HTTP transport not enabled - streamable-http feature missing");
        tracing::info!("Falling back to stdio transport");
        tracing::info!("To enable HTTP: cargo build --features streamable-http");
        run_pmcp_server().await?;
    }

    Ok(())
}
