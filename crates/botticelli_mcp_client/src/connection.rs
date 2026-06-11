//! Connection types for the thin Botticelli MCP client.

use crate::error::{McpClientError, McpClientErrorKind, McpClientResult};
use crate::handler::TuiHandler;
use rmcp::RoleClient;
use rmcp::model::{CallToolRequestParams, CallToolResult, ListToolsResult};
use rmcp::service::{Peer, RunningService};
use std::process::Stdio;
use std::sync::Arc;
use tracing::instrument;

/// Thin rmcp client for the Botticelli MCP server.
///
/// Holds the server connection and the running service that keeps it alive.
/// Use [`connect_stdio`](Self::connect_stdio) to spawn the server and connect.
pub struct BotticelliClient {
    peer: Arc<Peer<RoleClient>>,
    _service: RunningService<RoleClient, TuiHandler>,
    _child: Option<tokio::process::Child>,
}

impl BotticelliClient {
    /// Spawn `command serve` as a subprocess and connect via stdio.
    ///
    /// The child process lives as long as this client. The [`TuiHandler`]
    /// handles elicitation prompts by printing them to stdout and reading
    /// the user's answer from stdin.
    #[instrument(fields(command = %command.as_ref()))]
    pub async fn connect_stdio(command: impl AsRef<str>) -> McpClientResult<Self> {
        let command = command.as_ref();
        tracing::info!("Spawning Botticelli server");

        let mut child = tokio::process::Command::new(command)
            .args(["serve"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| McpClientError::new(McpClientErrorKind::ConnectionError(e.to_string())))?;

        let server_stdin = child.stdin.take().expect("stdin was piped");
        let server_stdout = child.stdout.take().expect("stdout was piped");

        tracing::debug!("Performing MCP handshake");
        let service = rmcp::serve_client(TuiHandler::new(), (server_stdout, server_stdin))
            .await
            .map_err(|e| McpClientError::new(McpClientErrorKind::ConnectionError(e.to_string())))?;

        let peer = service.peer().clone().into();
        tracing::info!("Connected to Botticelli server");

        Ok(Self {
            peer,
            _service: service,
            _child: Some(child),
        })
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
    #[instrument(skip(self))]
    pub fn peer(&self) -> &Peer<RoleClient> {
        &self.peer
    }
}
