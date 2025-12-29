//! RMCP-based MCP server implementation.
//!
//! The BotticelliServer struct holds all tool implementations and state
//! needed for MCP operations.

use crate::dialog_resource::DialogResource;
use crate::{EchoParams, EchoResult, ToolError};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use std::sync::Arc;
use tracing::{debug, instrument};

#[cfg(feature = "database")]
use botticelli_interface::DatabaseRegistryOperations;

/// Botticelli MCP server using rmcp.
///
/// This server exposes Botticelli's capabilities as MCP tools.
/// Use the builder pattern to construct instances with optional components.
///
/// # Examples
///
/// ```no_run
/// use botticelli_mcp::BotticelliServer;
///
/// let server = BotticelliServer::builder()
///     .build();
/// ```
#[derive(Clone)]
pub struct BotticelliServer {
    tool_router: ToolRouter<Self>,
    
    #[cfg(feature = "database")]
    db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,
    
    dialog: Option<Arc<DialogResource>>,
}

impl BotticelliServer {
    /// Create a builder for configuring the server.
    ///
    /// # Returns
    ///
    /// A new `BotticelliServerBuilder` with default configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use botticelli_mcp::BotticelliServer;
    ///
    /// let server = BotticelliServer::builder()
    ///     .build();
    /// ```
    pub fn builder() -> BotticelliServerBuilder {
        BotticelliServerBuilder::default()
    }
}

/// Builder for BotticelliServer.
///
/// Provides a type-safe way to configure optional server components
/// before construction.
#[derive(Clone, Default)]
pub struct BotticelliServerBuilder {
    #[cfg(feature = "database")]
    db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,
    
    dialog: Option<Arc<DialogResource>>,
}

impl BotticelliServerBuilder {
    /// Configure database operations.
    ///
    /// # Arguments
    ///
    /// * `db` - Database operations implementation
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[cfg(feature = "database")]
    pub fn database(mut self, db: Arc<dyn DatabaseRegistryOperations>) -> Self {
        self.db_ops = Some(db);
        self
    }
    
    /// Configure dialog resource for elicitation tools.
    ///
    /// # Arguments
    ///
    /// * `dialog` - Dialog resource for user interaction
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    pub fn dialog(mut self, dialog: Arc<DialogResource>) -> Self {
        self.dialog = Some(dialog);
        self
    }
    
    /// Build the BotticelliServer instance.
    ///
    /// # Returns
    ///
    /// A configured `BotticelliServer` ready to serve MCP requests.
    pub fn build(self) -> BotticelliServer {
        BotticelliServer {
            tool_router: BotticelliServer::tool_router(),
            #[cfg(feature = "database")]
            db_ops: self.db_ops,
            dialog: self.dialog,
        }
    }
}

#[tool_router]
impl BotticelliServer {
    /// Echo back a message with timestamp.
    ///
    /// This tool is useful for testing MCP connectivity and verifying
    /// that the server is responding correctly.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the message to echo
    ///
    /// # Returns
    ///
    /// Returns the echoed message with a timestamp on success.
    ///
    /// # Errors
    ///
    /// This tool should not fail under normal circumstances.
    #[tool(description = "Echoes back the input message with a timestamp")]
    #[instrument(skip(self), fields(message))]
    async fn echo(
        &self,
        Parameters(EchoParams { message }): Parameters<EchoParams>
    ) -> Result<Json<EchoResult>, rmcp::ErrorData> {
        debug!(?message, "Processing echo request");
        
        let result = EchoResult::new(message);
        
        debug!(result = ?result, "Echo completed successfully");
        Ok(Json(result))
    }
}

#[tool_handler]
impl ServerHandler for BotticelliServer {
    fn get_info(&self) -> rmcp::model::InitializeResult {
        rmcp::model::InitializeResult {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: rmcp::model::Implementation {
                name: "botticelli".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Botticelli".to_string()),
                website_url: None,
                icons: None,
            },
            instructions: Some("Botticelli MCP server - LLM orchestration tools".to_string()),
        }
    }
}
