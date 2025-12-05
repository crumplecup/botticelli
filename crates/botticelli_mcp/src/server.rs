//! MCP server implementation.

use crate::tools::ToolRegistry;
use crate::McpResult;
use tracing::{info, instrument};

/// MCP server for Botticelli.
pub struct McpServer {
    name: String,
    version: String,
    tools: ToolRegistry,
}

impl McpServer {
    /// Creates a new server builder.
    pub fn builder() -> McpServerBuilder {
        McpServerBuilder::default()
    }

    /// Runs the server using stdio transport.
    #[instrument(skip(self))]
    pub async fn run_stdio(self) -> McpResult<()> {
        info!(
            name = %self.name,
            version = %self.version,
            tools = self.tools.list().len(),
            "MCP server ready (stdio transport not yet fully implemented)"
        );

        // TODO: Integrate with rust-mcp-sdk stdio transport
        // For now, just log that we're ready
        
        Ok(())
    }
}

/// Builder for MCP server.
#[derive(Default)]
pub struct McpServerBuilder {
    name: Option<String>,
    version: Option<String>,
    tools: Option<ToolRegistry>,
}

impl McpServerBuilder {
    /// Sets the server name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the server version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the tool registry.
    pub fn tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Builds the server.
    pub fn build(self) -> McpResult<McpServer> {
        Ok(McpServer {
            name: self.name.unwrap_or_else(|| "botticelli".to_string()),
            version: self.version.unwrap_or_else(|| "0.1.0".to_string()),
            tools: self.tools.unwrap_or_default(),
        })
    }
}
