//! PMCP-based MCP server implementation.
//!
//! This is the new implementation using the pmcp SDK, which will eventually
//! replace the custom mcp-server implementation.

use anyhow::Result;
use async_trait::async_trait;
use pmcp::{RequestHandlerExtra, Server, ToolHandler};
use serde_json::Value;
use tracing::{info, instrument};

/// Tool handler for echo functionality.
pub struct EchoHandler;

#[async_trait]
impl ToolHandler for EchoHandler {
    #[instrument(skip(self, _extra))]
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> pmcp::Result<Value> {
        info!("Echo handler called with args: {:?}", args);

        // Extract message from args
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| pmcp::Error::validation("message field required"))?;

        // Return echo response
        Ok(serde_json::json!({
            "echo": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }
}

/// Runs the PMCP-based MCP server.
#[instrument]
pub async fn run_pmcp_server() -> Result<()> {
    info!("Starting PMCP-based MCP server");

    // Build server with echo tool
    let server = Server::builder()
        .name("botticelli-pmcp")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(pmcp::types::capabilities::ServerCapabilities::tools_only())
        .tool("echo", EchoHandler)
        .build()?;

    info!("Server built, running on stdio");

    // Run on stdio transport
    server.run_stdio().await?;

    Ok(())
}
