//! HTTP transport implementation for MCP.

use super::{McpTransport, McpTransportError, McpTransportErrorKind};
use async_trait::async_trait;
use botticelli_core::ToolDefinition;
use reqwest::Client;
use serde_json::Value;

/// HTTP-based MCP transport.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct HttpTransport {
    /// Base URL of the MCP server
    base_url: String,
    /// HTTP client
    #[getter(skip)]
    client: Client,
    /// Connection status
    connected: bool,
}

impl HttpTransport {
    /// Creates a new HTTP transport.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            connected: false,
        }
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    #[tracing::instrument(skip(self))]
    async fn initialize(&mut self) -> Result<(), McpTransportError> {
        // Test connection with a health check
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                McpTransportError::new(McpTransportErrorKind::ConnectionFailed(e.to_string()))
            })?;

        self.connected = true;
        tracing::info!("HTTP transport initialized");
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn list_tools(&self) -> Result<Vec<ToolDefinition>, McpTransportError> {
        if !self.connected {
            return Err(McpTransportError::new(McpTransportErrorKind::NotInitialized));
        }

        let url = format!("{}/tools/list", self.base_url);
        let response = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| {
                McpTransportError::new(McpTransportErrorKind::RequestFailed(e.to_string()))
            })?;

        let tools: Vec<ToolDefinition> = response
            .json()
            .await
            .map_err(|e| {
                McpTransportError::new(McpTransportErrorKind::InvalidResponse(e.to_string()))
            })?;

        tracing::debug!(count = tools.len(), "Listed tools");
        Ok(tools)
    }

    #[tracing::instrument(skip(self, arguments))]
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, McpTransportError> {
        if !self.connected {
            return Err(McpTransportError::new(McpTransportErrorKind::NotInitialized));
        }

        let url = format!("{}/tools/call", self.base_url);
        let request_body = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                McpTransportError::new(McpTransportErrorKind::RequestFailed(e.to_string()))
            })?;

        let result: Value = response
            .json()
            .await
            .map_err(|e| {
                McpTransportError::new(McpTransportErrorKind::InvalidResponse(e.to_string()))
            })?;

        tracing::debug!(tool = name, "Tool executed");
        Ok(result)
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
