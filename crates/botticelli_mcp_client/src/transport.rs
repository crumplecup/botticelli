use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use async_trait::async_trait;
use derive_getters::Getters;
use serde_json::Value;
use std::fmt::Debug;

/// Transport abstraction for MCP communication.
#[async_trait]
pub trait Transport: Send + Sync + Debug {
    /// Send a request and receive a response.
    async fn send(&self, request: Value) -> McpClientResult<Value>;

    /// Close the transport connection.
    async fn close(&self) -> McpClientResult<()>;

    /// Check if transport is connected.
    fn is_connected(&self) -> bool;
}

/// Stdio transport for local MCP servers.
#[derive(Debug, Getters)]
pub struct StdioTransport {
    command: String,
    args: Vec<String>,
    connected: bool,
}

impl StdioTransport {
    /// Create a new stdio transport.
    pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            connected: false,
        }
    }

    /// Builder for stdio transport.
    pub fn builder() -> StdioTransportBuilder {
        StdioTransportBuilder::default()
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&self, _request: Value) -> McpClientResult<Value> {
        if !self.connected {
            return Err(McpClientError::new(McpClientErrorKind::ConnectionError(
                "Transport not connected".to_string(),
            )));
        }
        
        // TODO: Implement actual stdio communication
        // This will use tokio::process::Command with stdin/stdout pipes
        todo!("Implement stdio transport send")
    }

    async fn close(&self) -> McpClientResult<()> {
        // TODO: Implement graceful shutdown
        todo!("Implement stdio transport close")
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Builder for StdioTransport.
#[derive(Debug, Default)]
pub struct StdioTransportBuilder {
    command: Option<String>,
    args: Vec<String>,
}

impl StdioTransportBuilder {
    /// Set the command to execute.
    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Add an argument to the command.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments to the command.
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args.extend(args);
        self
    }

    /// Build the stdio transport.
    pub fn build(self) -> McpClientResult<StdioTransport> {
        let command = self.command.ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::ConnectionError(
                "Command is required".to_string(),
            ))
        })?;

        Ok(StdioTransport::new(command, self.args))
    }
}

/// HTTP/SSE transport for remote MCP servers.
#[derive(Debug)]
pub struct HttpTransport {
    url: String,
    client: reqwest::Client,
    connected: bool,
}

impl HttpTransport {
    /// Create a new HTTP transport.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            client: reqwest::Client::new(),
            connected: false,
        }
    }

    /// Builder for HTTP transport.
    pub fn builder() -> HttpTransportBuilder {
        HttpTransportBuilder::default()
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn send(&self, request: Value) -> McpClientResult<Value> {
        if !self.connected {
            return Err(McpClientError::new(McpClientErrorKind::ConnectionError(
                "Transport not connected".to_string(),
            )));
        }

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::ConnectionError(format!(
                    "HTTP request failed: {}",
                    e
                )))
            })?;

        let value = response.json().await.map_err(|e| {
            McpClientError::new(McpClientErrorKind::SerializationError(format!(
                "Failed to parse response: {}",
                e
            )))
        })?;

        Ok(value)
    }

    async fn close(&self) -> McpClientResult<()> {
        // HTTP is stateless, nothing to close
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Builder for HttpTransport.
#[derive(Debug, Default)]
pub struct HttpTransportBuilder {
    url: Option<String>,
}

impl HttpTransportBuilder {
    /// Set the URL for the HTTP transport.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Build the HTTP transport.
    pub fn build(self) -> McpClientResult<HttpTransport> {
        let url = self.url.ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::ConnectionError(
                "URL is required".to_string(),
            ))
        })?;

        Ok(HttpTransport::new(url))
    }
}
