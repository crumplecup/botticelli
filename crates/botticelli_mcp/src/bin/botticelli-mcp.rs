//! Botticelli MCP server binary.

use anyhow::Result;
use botticelli_mcp::BotticelliServer;
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Botticelli MCP server — expose LLM and Discord tools over MCP.
#[derive(Parser, Debug)]
#[command(name = "botticelli-mcp", about = "Botticelli MCP server", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run over stdio (for Claude Desktop and MCP clients that spawn the process).
    Serve,

    /// Run an HTTP server (multi-client; the TUI connects here by default).
    Http {
        /// Host to bind to.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to bind to.
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Serve => run_stdio().await,
        Command::Http { host, port } => run_http(host, port).await,
    }
}

#[tracing::instrument]
async fn run_stdio() -> Result<()> {
    info!("Starting Botticelli MCP server (stdio)");
    let service =
        rmcp::service::serve_server(BotticelliServer::new(), rmcp::transport::stdio()).await?;
    service.waiting().await.ok();
    Ok(())
}

#[tracing::instrument(fields(host = %host, port))]
async fn run_http(host: String, port: u16) -> Result<()> {
    use axum::{Router, body::Body, http::Request};
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager,
        tower::{StreamableHttpServerConfig, StreamableHttpService},
    };
    use std::sync::Arc;
    use tower::ServiceBuilder;

    info!("Starting Botticelli MCP server (HTTP)");

    let config = StreamableHttpServerConfig::default().with_stateful_mode(true);
    let http_service = StreamableHttpService::new(
        || Ok(BotticelliServer::new()),
        Arc::new(LocalSessionManager::default()),
        config,
    );

    let app = Router::new()
        .route("/health", axum::routing::get(|| async { "OK" }))
        .fallback_service(ServiceBuilder::new().service(tower::service_fn(
            move |req: Request<Body>| {
                let mut service = http_service.clone();
                async move { tower::Service::call(&mut service, req).await }
            },
        )));

    let listener = tokio::net::TcpListener::bind((host.as_str(), port)).await?;
    info!("MCP server ready — http://{}:{}/mcp", host, port);
    axum::serve(listener, app).await?;
    Ok(())
}
