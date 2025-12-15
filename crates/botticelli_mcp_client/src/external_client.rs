//! External MCP server client for connecting to ecosystem servers.
//!
//! This module provides functionality to spawn and communicate with external
//! MCP servers like filesystem, git, search, etc.

use crate::tool_executor::ToolDefinition;
use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use serde_json::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, info, instrument};
use typed_builder::TypedBuilder;

/// Configuration for connecting to an external MCP server.
#[derive(Debug, Clone, TypedBuilder)]
pub struct ExternalServerConfig {
    /// Server identifier (e.g., "filesystem", "git")
    pub name: String,

    /// Command to execute (e.g., "npx", "mcp-server-git")
    pub command: String,

    /// Arguments to pass to command
    pub args: Vec<String>,

    /// Optional: Restrict which tools can be used
    #[builder(default)]
    pub allowed_tools: Option<Vec<String>>,

    /// Optional: Timeout for operations (seconds)
    #[builder(default)]
    pub timeout_seconds: Option<u64>,
}

/// Client for connecting to external MCP servers (filesystem, git, search, etc.)
///
/// This is Phase B.1 initial implementation - basic process spawning and communication.
/// TODO Phase B.2: Integrate with pmcp SDK once we understand the transport layer better.
pub struct ExternalMcpClient {
    /// Server identifier
    name: String,

    /// Child process
    _child: Child,

    /// Standard input to child process
    stdin: ChildStdin,

    /// Standard output from child process
    stdout: BufReader<ChildStdout>,

    /// Available tools (discovered on init)
    tools: Vec<ToolDefinition>,

    /// Allowed tools (if restricted)
    allowed_tools: Option<Vec<String>>,

    /// Call count for metrics
    call_count: AtomicU64,
}

impl ExternalMcpClient {
    /// Connect to an external MCP server by spawning the process.
    ///
    /// NOTE: This is a Phase B.1 implementation. Currently spawns the process but
    /// needs full MCP protocol implementation.
    #[instrument(skip(config), fields(server_name = %config.name))]
    pub async fn connect(config: ExternalServerConfig) -> McpClientResult<Self> {
        info!(
            "Spawning external MCP server: {} (command: {} {:?})",
            config.name, config.command, config.args
        );

        // Spawn the external process
        let mut child = Command::new(&config.command)
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Inherit stderr for debugging
            .spawn()
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::ExternalServerConnectionFailed(format!(
                    "Failed to spawn process '{}': {}",
                    config.command, e
                )))
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::ExternalServerConnectionFailed(
                "Failed to get stdin handle".to_string(),
            ))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::ExternalServerConnectionFailed(
                "Failed to get stdout handle".to_string(),
            ))
        })?;

        let stdout = BufReader::new(stdout);

        info!("Process spawned successfully: {}", config.name);

        // TODO Phase B.2: Send initialize request via MCP protocol
        // TODO Phase B.2: Discover tools via tools/list request
        // For now, return with empty tool list

        Ok(Self {
            name: config.name,
            _child: child,
            stdin,
            stdout,
            tools: Vec::new(), // TODO: Discover from server
            allowed_tools: config.allowed_tools,
            call_count: AtomicU64::new(0),
        })
    }

    /// Get server name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get available tools from this server.
    pub fn tools(&self) -> Vec<ToolDefinition> {
        self.tools.clone()
    }

    /// Check if server has a specific tool.
    pub fn has_tool(&self, tool_name: &str) -> bool {
        // Check allowed list first
        if let Some(allowed) = &self.allowed_tools {
            if !allowed.contains(&tool_name.to_string()) {
                return false;
            }
        }

        self.tools.iter().any(|t| t.name == tool_name)
    }

    /// Call a tool on the external server.
    ///
    /// TODO Phase B.2: Implement MCP protocol tool calling
    #[instrument(skip(self, _arguments), fields(server = %self.name, tool = %tool_name))]
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        _arguments: Value,
    ) -> McpClientResult<Value> {
        // Verify tool exists and is allowed
        if !self.has_tool(tool_name) {
            return Err(McpClientError::new(McpClientErrorKind::ToolNotFound(
                format!(
                    "Tool '{}' not available on server '{}'",
                    tool_name, self.name
                ),
            )));
        }

        debug!(
            "Calling tool '{}' on server '{}' (NOT YET IMPLEMENTED)",
            tool_name, self.name
        );

        // TODO Phase B.2: Send tool call request via MCP protocol
        // TODO Phase B.2: Parse response

        // Track metrics
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // Placeholder return
        Err(McpClientError::new(
            McpClientErrorKind::ToolExecutionFailed(
                "External tool calling not yet implemented - Phase B.2 pending".to_string(),
            ),
        ))
    }

    /// Get call statistics.
    pub fn call_count(&self) -> u64 {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_server_config_builder() {
        let config = ExternalServerConfig::builder()
            .name("filesystem".to_string())
            .command("npx".to_string())
            .args(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                "/tmp".to_string(),
            ])
            .allowed_tools(Some(vec!["read_file".to_string()]))
            .timeout_seconds(Some(30))
            .build();

        assert_eq!(config.name, "filesystem");
        assert_eq!(config.command, "npx");
        assert_eq!(config.args.len(), 3);
        assert!(config.allowed_tools.is_some());
        assert_eq!(config.timeout_seconds, Some(30));
    }

    #[tokio::test]
    async fn test_spawn_echo_process() {
        // Test that we can spawn a simple process
        let config = ExternalServerConfig::builder()
            .name("echo".to_string())
            .command("echo".to_string())
            .args(vec!["test".to_string()])
            .build();

        // This will spawn but won't be a valid MCP server
        // Just tests process spawning works
        let result = ExternalMcpClient::connect(config).await;
        
        // Should successfully spawn even though it's not an MCP server
        assert!(result.is_ok(), "Should spawn process: {:?}", result.err());
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_server_config_builder() {
        let config = ExternalServerConfig::builder()
            .name("filesystem".to_string())
            .command("npx".to_string())
            .args(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                "/tmp".to_string(),
            ])
            .allowed_tools(Some(vec!["read_file".to_string()]))
            .timeout_seconds(Some(30))
            .build();

        assert_eq!(config.name, "filesystem");
        assert_eq!(config.command, "npx");
        assert_eq!(config.args.len(), 3);
        assert!(config.allowed_tools.is_some());
        assert_eq!(config.timeout_seconds, Some(30));
    }

    #[tokio::test]
    async fn test_spawn_echo_process() {
        // Test that we can spawn a simple process
        let config = ExternalServerConfig::builder()
            .name("echo".to_string())
            .command("echo".to_string())
            .args(vec!["test".to_string()])
            .build();

        // This will spawn but won't be a valid MCP server
        // Just tests process spawning works
        let result = ExternalMcpClient::connect(config).await;
        
        // Should successfully spawn even though it's not an MCP server
        assert!(result.is_ok(), "Should spawn process: {:?}", result.err());
    }
}
