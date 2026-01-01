//! Connection helpers for MCP servers using rmcp.

use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::ServiceExt;
use tokio::process::Command;
use tracing::instrument;

/// Connect to an MCP server via child process (stdio).
#[instrument(skip_all, fields(command, args_count = args.len()))]
pub async fn connect_stdio(
    command: &str,
    args: Vec<String>,
) -> McpClientResult<RunningService<RoleClient, ()>> {
    let child_process = TokioChildProcess::new(Command::new(command).configure(|cmd| {
        for arg in args {
            cmd.arg(arg);
        }
    }))
    .map_err(|e| {
        McpClientError::new(McpClientErrorKind::ConnectionError(format!(
            "Failed to spawn child process: {}",
            e
        )))
    })?;

    let service = ()
        .serve(child_process)
        .await
        .map_err(|e| {
            McpClientError::new(McpClientErrorKind::ConnectionError(format!(
                "Failed to connect via stdio: {}",
                e
            )))
        })?;

    tracing::info!(command, "Connected to MCP server via stdio");
    Ok(service)
}
