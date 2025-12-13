//! Botticelli MCP HTTP server binary.
//!
//! Runs the MCP server over HTTP instead of stdio for web access.

use anyhow::Result;
use botticelli_mcp::{BotticelliRouter, NarrativeResource, ResourceRegistry};
use std::sync::Arc;
use tracing_subscriber::{self, EnvFilter};

#[cfg(feature = "database")]
use botticelli_mcp::ContentResource;

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

    tracing::info!("Starting Botticelli MCP HTTP server");

    // Register resources
    let mut resources = ResourceRegistry::new();

    #[cfg(feature = "database")]
    resources.register(Arc::new(ContentResource::new()));

    resources.register(Arc::new(NarrativeResource::new()));

    // Create router with default tools and resources
    let router = BotticelliRouter::builder()
        .name("botticelli")
        .version(env!("CARGO_PKG_VERSION"))
        .resources(resources)
        .build();

    use botticelli_mcp::Router;
    tracing::info!(
        tools = router.list_tools().len(),
        resources = router.list_resources().len(),
        "Router initialized"
    );

    // Get port from environment or use default
    let port = std::env::var("MCP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    // Start HTTP server
    botticelli_mcp::http::create_server(router, port).await?;

    Ok(())
}
