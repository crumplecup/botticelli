use serde::{Deserialize, Serialize};

use crate::EnvironmentMode;

/// MCP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// MCP server host.
    pub host: String,

    /// MCP server port.
    pub port: u16,
}

impl McpServerConfig {
    /// Create config with environment-aware defaults.
    pub fn with_defaults(mode: EnvironmentMode) -> Self {
        Self {
            host: mode.mcp_server_host_default().to_string(),
            port: mode.mcp_server_port_default(),
        }
    }

    /// Build MCP server URL from configuration.
    pub fn server_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self::with_defaults(EnvironmentMode::Local)
    }
}

/// MCP client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientConfig {
    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Number of retry attempts.
    #[serde(default = "default_retries")]
    pub retry_attempts: u32,
}

fn default_timeout() -> u64 {
    30
}

fn default_retries() -> u32 {
    3
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: default_timeout(),
            retry_attempts: default_retries(),
        }
    }
}
