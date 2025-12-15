//! External MCP server client for connecting to ecosystem servers.
//!
//! This module provides functionality to spawn and communicate with external
//! MCP servers like filesystem, git, search, etc.

use crate::tool_executor::ToolDefinition;
use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use pmcp::{Client, ClientCapabilities, Transport};
use pmcp::types::TransportMessage;
use serde_json::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tracing::{debug, info, instrument};
use typed_builder::TypedBuilder;

/// Custom transport for external child process communication.
#[derive(Debug)]
struct ChildProcessTransport {
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
}

impl ChildProcessTransport {
    fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
        }
    }
}

#[async_trait::async_trait]
impl Transport for ChildProcessTransport {
    async fn send(&mut self, message: TransportMessage) -> pmcp::Result<()> {
        let mut stdin = self.stdin.lock().await;
        let json = serde_json::to_string(&message)
            .map_err(|e| pmcp::Error::internal(format!("Failed to serialize message: {}", e)))?;
        
        stdin
            .write_all(json.as_bytes())
            .await
            .map_err(|e| pmcp::Error::internal(format!("Failed to write to stdin: {}", e)))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| pmcp::Error::internal(format!("Failed to write newline: {}", e)))?;
        stdin
            .flush()
            .await
            .map_err(|e| pmcp::Error::internal(format!("Failed to flush stdin: {}", e)))?;
        Ok(())
    }

    async fn receive(&mut self) -> pmcp::Result<TransportMessage> {
        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();
        stdout
            .read_line(&mut line)
            .await
            .map_err(|e| pmcp::Error::internal(format!("Failed to read from stdout: {}", e)))?;
        
        serde_json::from_str(&line)
            .map_err(|e| pmcp::Error::parse(format!("Failed to parse message: {}", e)))
    }

    async fn close(&mut self) -> pmcp::Result<()> {
        Ok(())
    }
}

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
#[derive(Debug)]
pub struct ExternalMcpClient {
    /// Server identifier
    name: String,

    /// Child process
    _child: Child,

    /// pmcp client for MCP protocol communication
    client: Client<ChildProcessTransport>,

    /// Available tools (discovered on init)
    tools: Vec<ToolDefinition>,

    /// Allowed tools (if restricted)
    allowed_tools: Option<Vec<String>>,

    /// Call count for metrics
    call_count: AtomicU64,
}

impl ExternalMcpClient {
    /// Connect to an external MCP server by spawning the process.
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
            .stderr(Stdio::inherit())
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

        info!("Process spawned successfully: {}", config.name);

        // Create custom transport and pmcp client
        let transport = ChildProcessTransport::new(stdin, stdout);
        let mut client = Client::new(transport);

        // Initialize MCP connection
        let capabilities = ClientCapabilities::minimal();
        let server_info = client
            .initialize(capabilities)
            .await
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::ExternalServerConnectionFailed(format!(
                    "Failed to initialize MCP connection with {}: {}",
                    config.name, e
                )))
            })?;

        info!(
            "MCP connection established with {} (version {})",
            server_info.server_info.name, server_info.server_info.version
        );

        // Discover available tools
        let tools_result = client.list_tools(None).await.map_err(|e| {
            McpClientError::new(McpClientErrorKind::ExternalServerDiscoveryFailed(format!(
                "Failed to list tools from {}: {}",
                config.name, e
            )))
        })?;

        info!(
            "Discovered {} tools from {}",
            tools_result.tools.len(),
            config.name
        );

        // Convert pmcp tool definitions to our format
        let tools: Vec<ToolDefinition> = tools_result
            .tools
            .into_iter()
            .map(|t| ToolDefinition {
                name: t.name,
                description: t.description.unwrap_or_default(),
                input_schema: t.input_schema,
            })
            .collect();

        for tool in &tools {
            debug!("  - {} ({})", tool.name, tool.description);
        }

        Ok(Self {
            name: config.name,
            _child: child,
            client,
            tools,
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
    #[instrument(skip(self, arguments), fields(server = %self.name, tool = %tool_name))]
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> McpClientResult<Value> {
        // Verify tool exists and is allowed
        if !self.has_tool(tool_name) {
            return Err(McpClientError::new(McpClientErrorKind::ToolNotFound(
                format!(
                    "Tool '{}' not available on server '{}' (either doesn't exist or not in allowed list)",
                    tool_name, self.name
                ),
            )));
        }

        debug!(
            "Calling tool '{}' on server '{}' with args: {}",
            tool_name, self.name, arguments
        );

        // Call via pmcp client
        let result = self
            .client
            .call_tool(tool_name.to_string(), arguments)
            .await
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!(
                    "Tool '{}' failed on server '{}': {}",
                    tool_name, self.name, e
                )))
            })?;

        // Track metrics
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // Convert Content to JSON Value
        // The content is typically a Vec<Content>, serialize it
        serde_json::to_value(result.content).map_err(|e| {
            McpClientError::new(McpClientErrorKind::SerializationError(format!(
                "Failed to serialize tool result: {}",
                e
            )))
        })
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
    async fn test_spawn_invalid_server() {
        // Test that we get proper error when spawning non-MCP process
        let config = ExternalServerConfig::builder()
            .name("not-mcp".to_string())
            .command("echo".to_string())
            .args(vec!["test".to_string()])
            .build();

        let result = ExternalMcpClient::connect(config).await;
        
        // Should fail because echo is not an MCP server
        assert!(result.is_err(), "Should fail with non-MCP process");
        
        let err = result.unwrap_err();
        assert!(
            format!("{}", err).contains("initialize") ||
            format!("{}", err).contains("Protocol") ||
            format!("{}", err).contains("parse"),
            "Error should indicate protocol/initialization failure: {}",
            err
        );
    }
}



