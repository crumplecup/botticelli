//! Botticelli MCP server binary.

use anyhow::Result;
use botticelli_mcp::{
    BotticelliRouter, ByteTransport, NarrativeResource, ResourceRegistry, Router, RouterService,
    Server,
};
use std::sync::Arc;
use tokio::io::{stdin, stdout};
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

    tracing::info!("Starting Botticelli MCP server");

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

    tracing::info!(
        tools = router.list_tools().len(),
        resources = router.list_resources().len(),
        "Router initialized"
    );

    // Create and run server with stdio transport
    let server = Server::new(RouterService(router));
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server ready, listening on stdio");
    server.run(transport).await?;

    Ok(())
}
