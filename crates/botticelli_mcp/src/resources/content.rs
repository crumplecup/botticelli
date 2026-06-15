//! Content resource for BotStorage content.

use super::{McpResource, ResourceInfo};
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use botticelli_interface::BotStorage;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Resource for accessing stored content via BotStorage.
///
/// URI format: `content://{table}/{id}`
/// Example: `content://content/550e8400-e29b-41d4-a716-446655440000`
pub struct ContentResource {
    storage: Arc<dyn BotStorage>,
}

impl ContentResource {
    /// Creates a new content resource backed by the given storage.
    pub fn new(storage: Arc<dyn BotStorage>) -> Self {
        Self { storage }
    }

    /// Parses a content URI into (table, id).
    #[instrument(skip(self))]
    fn parse_uri(&self, uri: &str) -> McpResult<(String, String)> {
        let without_scheme = uri.strip_prefix("content://").ok_or_else(|| {
            McpError::resource_not_found(
                "Invalid content URI: missing content:// scheme".to_string(),
            )
        })?;

        let parts: Vec<&str> = without_scheme.splitn(2, '/').collect();
        if parts.len() != 2 || parts[1].is_empty() {
            return Err(McpError::invalid_input(format!(
                "Invalid content URI format. Expected content://table/id, got {}",
                uri
            )));
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Retrieves a single content record by table and id.
    #[instrument(skip(self))]
    async fn query_content(&self, table: &str, id: &str) -> McpResult<serde_json::Value> {
        let record = self
            .storage
            .get_content(id)
            .await
            .map_err(|e| McpError::execution_failed(format!("Storage error: {}", e)))?;

        record
            .map(|r| r.content_json)
            .ok_or_else(|| McpError::resource_not_found(format!("Content not found: {}/{}", table, id)))
    }
}

#[async_trait]
impl McpResource for ContentResource {
    fn uri_pattern(&self) -> &'static str {
        "content://"
    }

    fn description(&self) -> &'static str {
        "Access stored content by table and ID"
    }

    #[instrument(skip(self), fields(uri))]
    async fn read(&self, uri: &str) -> McpResult<String> {
        let (table, id) = self.parse_uri(uri)?;
        debug!(table, id, "Reading content");

        let content = self.query_content(&table, &id).await?;

        serde_json::to_string_pretty(&content)
            .map_err(|e| McpError::execution_failed(format!("Failed to serialize content: {}", e)))
    }

    #[instrument(skip(self))]
    async fn list(&self) -> McpResult<Vec<ResourceInfo>> {
        let rows = self
            .storage
            .list_content("content", 20)
            .await
            .map_err(|e| McpError::execution_failed(format!("Failed to list content: {}", e)))?;

        let resources = rows
            .into_iter()
            .map(|row| {
                let text = row
                    .content_json
                    .get("text_content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let preview = if text.len() > 50 {
                    format!("{}...", &text[..50])
                } else {
                    text.to_string()
                };

                ResourceInfo {
                    uri: format!("content://content/{}", row.id),
                    name: format!("Content {}", row.id),
                    description: preview,
                    mime_type: Some("application/json".to_string()),
                }
            })
            .collect();

        Ok(resources)
    }
}
