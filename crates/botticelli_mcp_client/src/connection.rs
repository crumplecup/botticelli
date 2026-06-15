//! Connection types for the thin Botticelli MCP client.

use crate::error::{McpClientError, McpClientErrorKind, McpClientResult};
use crate::handler::TuiHandler;
use rmcp::RoleClient;
use rmcp::model::{CallToolRequestParams, CallToolResult, ListToolsResult};
use rmcp::service::{Peer, RunningService};
use std::sync::Arc;
use tracing::instrument;

/// Thin rmcp client for the Botticelli MCP server.
///
/// Holds the server connection and the running service that keeps it alive.
/// Use [`connect_http`](Self::connect_http) to reach an external server or
/// [`connect_duplex`](Self::connect_duplex) for an in-process server.
pub struct BotticelliClient {
    peer: Arc<Peer<RoleClient>>,
    _service: RunningService<RoleClient, TuiHandler>,
}

impl BotticelliClient {
    /// Connect to a Botticelli MCP server over Streamable HTTP.
    ///
    /// `url` should be the server's MCP endpoint, e.g. `http://localhost:3000/mcp`.
    #[instrument(fields(url = %url.as_ref()))]
    pub async fn connect_http(url: impl AsRef<str>) -> McpClientResult<Self> {
        use rmcp::transport::StreamableHttpClientTransport;

        let url = url.as_ref();
        tracing::info!("Connecting to Botticelli server via HTTP");

        let transport = StreamableHttpClientTransport::from_uri(url);
        let service = rmcp::serve_client(TuiHandler::new(), transport)
            .await
            .map_err(|e| McpClientError::new(McpClientErrorKind::ConnectionError(e.to_string())))?;

        let peer = service.peer().clone().into();
        tracing::info!("Connected to Botticelli server via HTTP");

        Ok(Self {
            peer,
            _service: service,
        })
    }

    /// Connect to an in-process MCP server via a [`tokio::io::DuplexStream`].
    ///
    /// The caller is responsible for spawning the server side before calling this.
    #[instrument]
    pub async fn connect_duplex(stream: tokio::io::DuplexStream) -> McpClientResult<Self> {
        tracing::info!("Connecting to in-process Botticelli server via duplex");

        let service = rmcp::serve_client(TuiHandler::new(), stream)
            .await
            .map_err(|e| McpClientError::new(McpClientErrorKind::ConnectionError(e.to_string())))?;

        let peer = service.peer().clone().into();
        tracing::info!("Connected to in-process Botticelli server");

        Ok(Self {
            peer,
            _service: service,
        })
    }

    /// Start a [`botticelli_mcp::BotticelliServer`] in-process and connect to it.
    ///
    /// No external process is needed — the server runs as a background task in the
    /// same tokio runtime. Use this as the default for the TUI.
    #[instrument]
    pub async fn connect_in_process() -> McpClientResult<Self> {
        use botticelli_mcp::BotticelliServer;

        tracing::info!("Spawning in-process BotticelliServer");

        let (server_io, client_io) = tokio::io::duplex(65_536);
        tokio::spawn(async move {
            match rmcp::serve_server(BotticelliServer::new(), server_io).await {
                Ok(service) => {
                    service.waiting().await.ok();
                }
                Err(e) => {
                    tracing::error!(error = %e, "In-process MCP server startup failed");
                }
            }
        });

        Self::connect_duplex(client_io).await
    }

    /// Call a tool on the server by name.
    #[instrument(skip(self, name, args), fields(tool))]
    pub async fn call_tool(
        &self,
        name: impl AsRef<str>,
        args: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> McpClientResult<CallToolResult> {
        tracing::Span::current().record("tool", name.as_ref());
        let params = match args {
            Some(a) => CallToolRequestParams::new(name.as_ref().to_string()).with_arguments(a),
            None => CallToolRequestParams::new(name.as_ref().to_string()),
        };
        self.peer.call_tool(params).await.map_err(|e| {
            McpClientError::new(McpClientErrorKind::ToolExecutionFailed(e.to_string()))
        })
    }

    /// List all tools the server currently exposes.
    #[instrument(skip(self))]
    pub async fn list_tools(&self) -> McpClientResult<ListToolsResult> {
        self.peer
            .list_tools(Default::default())
            .await
            .map_err(|e| McpClientError::new(McpClientErrorKind::ConnectionError(e.to_string())))
    }

    /// Return the underlying rmcp peer for direct low-level access.
    pub fn peer(&self) -> &Peer<RoleClient> {
        &self.peer
    }
}
