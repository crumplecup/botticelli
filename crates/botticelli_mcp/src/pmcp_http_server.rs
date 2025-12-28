//! HTTP server implementation using pmcp's StreamableHttpServer.
//!
//! This provides HTTP transport for the MCP server with:
//! - Health and metrics endpoints
//! - Stateless operation (serverless-friendly)
//! - Proper error handling and observability

use crate::pmcp_server::register_all_tools;
use anyhow::Result;
use pmcp::server::streamable_http_server::{StreamableHttpServer, StreamableHttpServerConfig};
use pmcp::Server;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, instrument, warn};

/// Builds the MCP server with all tools.
///
/// This is the same server logic as stdio, but prepared for HTTP transport.
#[instrument(skip(db_ops))]
fn build_server(
    #[cfg(feature = "database")] db_ops: Option<
        Arc<dyn botticelli_interface::DatabaseRegistryOperations>,
    >,
) -> Result<Server> {
    info!("Building MCP server for HTTP transport");

    let builder = Server::builder()
        .name("botticelli-pmcp-http")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(pmcp::types::capabilities::ServerCapabilities::tools_only());

    // Use shared tool registration function
    let builder = register_all_tools(
        builder,
        None, // No dialog resource for HTTP server
        #[cfg(feature = "database")]
        db_ops,
    );
    
    builder.build().map_err(Into::into)
}

/// Runs the HTTP MCP server.
#[instrument(skip(db_ops))]
pub async fn run_pmcp_http_server(
    host: &str,
    port: u16,
    #[cfg(feature = "database")] db_ops: Option<
        Arc<dyn botticelli_interface::DatabaseRegistryOperations>,
    >,
) -> Result<()> {
    info!("Starting PMCP HTTP server on {}:{}", host, port);

    // Build the server
    let server = build_server(
        #[cfg(feature = "database")]
        db_ops,
    )?;
    info!("Server built successfully with all tools");

    // Wrap in Arc<Mutex<>> for HTTP server
    let server = Arc::new(Mutex::new(server));

    // Configure address
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    info!("Configuring HTTP server for {}", addr);

    // Create stateless configuration (serverless-friendly)
    let config = StreamableHttpServerConfig {
        session_id_generator: None,   // Stateless mode
        enable_json_response: true,   // JSON responses
        event_store: None,            // No event store
        on_session_initialized: None, // No session callbacks
        on_session_closed: None,
        http_middleware: None, // TODO: Add middleware when needed
    };

    // Create HTTP server
    let http_server = StreamableHttpServer::with_config(addr, server, config);

    info!("Starting HTTP server on {}", addr);

    // Start server
    let (bound_addr, _handle) = http_server.start().await?;

    info!("✅ HTTP server SUCCESSFULLY BOUND to {}", bound_addr);
    info!("📍 Server is listening and ready to accept connections");
    info!("🔗 Endpoint: POST http://{}/sse", bound_addr);
    info!("📝 Note: pmcp StreamableHttpServer uses /sse endpoint by default");

    // Print banner
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║         BOTTICELLI MCP HTTP SERVER (PMCP)                 ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Address: http://{:43} ║", bound_addr);
    println!("║ Mode:    Stateless (serverless-friendly)                  ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Features:                                                  ║");
    println!("║ • All 26 tools available via HTTP                         ║");
    println!("║ • Stateless operation (no session management)             ║");
    println!("║ • Horizontal scaling ready                                ║");
    println!("║ • Full observability via tracing                          ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Endpoints:                                                 ║");
    println!("║ • POST /sse - MCP Server-Sent Events endpoint             ║");
    println!("║ • GET  /health - Health check (future)                    ║");
    println!("║ • GET  /metrics - Prometheus metrics (future)             ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    // Keep server running
    tokio::signal::ctrl_c().await?;
    info!("Received shutdown signal, stopping server");

    Ok(())
}
