//! Transport abstraction for MCP communication.

use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use async_trait::async_trait;
use botticelli_core::ToolDefinition;
use serde_json::Value;

/// Transport mechanism for MCP client-server communication.
#[async_trait]
pub trait McpTransport: Send + Sync + std::fmt::Debug {
    /// Initialize the transport connection.
    async fn connect(&mut self) -> McpClientResult<()>;

    /// Send a request and receive a response.
    async fn send_request(&mut self, method: &str, params: Value) -> McpClientResult<Value>;

    /// List available tools from the server.
    async fn list_tools(&mut self) -> McpClientResult<Vec<ToolDefinition>>;

    /// Execute a tool call.
    async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> McpClientResult<Value>;

    /// Close the transport connection.
    async fn disconnect(&mut self) -> McpClientResult<()>;
}

/// HTTP-based transport for MCP communication.
#[derive(Debug)]
pub struct HttpTransport {
    /// Base URL of the MCP server
    base_url: String,
    /// HTTP client
    client: reqwest::Client,
    /// Whether the connection is established
    connected: bool,
}

impl HttpTransport {
    /// Create a new HTTP transport.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
            connected: false,
        }
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn connect(&mut self) -> McpClientResult<()> {
        // Test connection with a tools/list request (PMCP server uses root path)
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        self.client
            .post(&self.base_url)
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::ConnectionError(format!(
                    "HTTP connection failed: {}",
                    e
                )))
            })?;

        self.connected = true;
        tracing::info!(base_url = %self.base_url, "HTTP transport connected");
        Ok(())
    }

    async fn send_request(&mut self, method: &str, params: Value) -> McpClientResult<Value> {
        if !self.connected {
            return Err(McpClientError::new(McpClientErrorKind::ConnectionError(
                "Not connected".to_string(),
            )));
        }

        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let response = self
            .client
            .post(&self.base_url) // PMCP uses root path
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::ConnectionError(format!(
                    "HTTP request failed: {}",
                    e
                )))
            })?;

        let response_json: Value = response.json().await.map_err(|e| {
            McpClientError::new(McpClientErrorKind::SerializationError(format!(
                "Failed to parse response: {}",
                e
            )))
        })?;

        // Extract result from JSON-RPC response
        response_json.get("result").cloned().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::SerializationError(
                "No result in response".to_string(),
            ))
        })
    }

    async fn list_tools(&mut self) -> McpClientResult<Vec<ToolDefinition>> {
        let result = self.send_request("tools/list", Value::Null).await?;

        let tools_array = result
            .get("tools")
            .and_then(|t| t.as_array())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::SerializationError(
                    "Invalid tools list response".to_string(),
                ))
            })?;

        let mut tools = Vec::new();
        for tool_value in tools_array {
            let tool: ToolDefinition = serde_json::from_value(tool_value.clone()).map_err(|e| {
                McpClientError::new(McpClientErrorKind::SerializationError(format!(
                    "Failed to parse tool definition: {}",
                    e
                )))
            })?;
            tools.push(tool);
        }

        Ok(tools)
    }

    async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> McpClientResult<Value> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        });

        self.send_request("tools/call", params).await
    }

    async fn disconnect(&mut self) -> McpClientResult<()> {
        self.connected = false;
        tracing::info!("HTTP transport disconnected");
        Ok(())
    }
}

/// Stdio-based transport for MCP communication (subprocess).
#[derive(Debug)]
pub struct StdioTransport {
    /// Command to execute
    command: String,
    /// Arguments for the command
    args: Vec<String>,
    /// Process handle (if connected)
    process: Option<tokio::process::Child>,
}

impl StdioTransport {
    /// Create a new stdio transport.
    pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            process: None,
        }
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn connect(&mut self) -> McpClientResult<()> {
        use tokio::process::Command;

        let child = Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::ConnectionError(format!(
                    "Failed to spawn subprocess: {}",
                    e
                )))
            })?;

        self.process = Some(child);
        tracing::info!(command = %self.command, "Stdio transport connected");
        Ok(())
    }

    async fn send_request(&mut self, _method: &str, _params: Value) -> McpClientResult<Value> {
        // TODO: Implement stdio JSON-RPC communication
        Err(McpClientError::new(McpClientErrorKind::Configuration(
            "Stdio transport not fully implemented".to_string(),
        )))
    }

    async fn list_tools(&mut self) -> McpClientResult<Vec<ToolDefinition>> {
        // TODO: Implement via send_request
        Err(McpClientError::new(McpClientErrorKind::Configuration(
            "Stdio transport not fully implemented".to_string(),
        )))
    }

    async fn call_tool(&mut self, _tool_name: &str, _arguments: Value) -> McpClientResult<Value> {
        // TODO: Implement via send_request
        Err(McpClientError::new(McpClientErrorKind::Configuration(
            "Stdio transport not fully implemented".to_string(),
        )))
    }

    async fn disconnect(&mut self) -> McpClientResult<()> {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill().await;
        }
        tracing::info!("Stdio transport disconnected");
        Ok(())
    }
}
