//! Discord HTTP client for MCP server tools.
//!
//! Provides the shared `DiscordClient` used by the `#[tool]` methods on
//! `BotticelliServer`.

#[cfg(feature = "discord")]
use botticelli_error::{McpError, McpResult};

#[cfg(feature = "discord")]
use serde_json::Value;

#[cfg(feature = "discord")]
use reqwest::Client;

#[cfg(feature = "discord")]
use tracing::{debug, instrument};

/// Discord API base URL.
#[cfg(feature = "discord")]
const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

/// Shared Discord HTTP client for all server tools.
#[cfg(feature = "discord")]
#[derive(Clone)]
pub(crate) struct DiscordClient {
    client: Client,
    token: String,
}

#[cfg(feature = "discord")]
impl DiscordClient {
    /// Creates a new Discord HTTP client.
    pub(crate) fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    /// Makes an authenticated GET request to Discord API.
    #[instrument(skip(self), fields(endpoint))]
    pub(crate) async fn get(&self, endpoint: &str) -> McpResult<Value> {
        let url = format!("{}{}", DISCORD_API_BASE, endpoint);
        debug!(url = %url, "Discord API GET");

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .header("User-Agent", "Botticelli-MCP/0.1.0")
            .send()
            .await
            .map_err(|e| {
                McpError::execution_failed(format!("Discord API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::execution_failed(format!(
                "Discord API error {}: {}",
                status, body
            )));
        }

        response.json().await.map_err(|e| {
            McpError::execution_failed(format!("Failed to parse Discord response: {}", e))
        })
    }

    /// Makes an authenticated POST request to Discord API.
    #[instrument(skip(self, body), fields(endpoint))]
    pub(crate) async fn post(&self, endpoint: &str, body: Value) -> McpResult<Value> {
        let url = format!("{}{}", DISCORD_API_BASE, endpoint);
        debug!(url = %url, "Discord API POST");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .header("User-Agent", "Botticelli-MCP/0.1.0")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                McpError::execution_failed(format!("Discord API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::execution_failed(format!(
                "Discord API error {}: {}",
                status, body
            )));
        }

        response.json().await.map_err(|e| {
            McpError::execution_failed(format!("Failed to parse Discord response: {}", e))
        })
    }
}
