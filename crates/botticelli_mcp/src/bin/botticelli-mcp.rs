//! Botticelli MCP server binary.

use anyhow::Result;
use botticelli_mcp::BotticelliServer;
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting Botticelli MCP server");

    let service =
        rmcp::service::serve_server(BotticelliServer::new(), rmcp::transport::stdio()).await?;
    service.waiting().await.ok();

    Ok(())
}
