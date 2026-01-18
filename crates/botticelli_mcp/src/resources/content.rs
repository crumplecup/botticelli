//! Content resource for database content.

use crate::ResourceInfo;
use async_trait::async_trait;
use botticelli_database::{establish_connection, get_content_by_id, list_content};
use botticelli_error::{McpError, McpResult};
use botticelli_interface::McpResource;
use tracing::{debug, instrument};

/// Resource for accessing database content.
///
/// URI format: `content://{table}/{id}`
/// Example: `content://content/123`
pub struct ContentResource;

impl ContentResource {
    /// Creates a new content resource.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self
    }

    /// Parses a content URI into (table, id).
    #[tracing::instrument(skip(self))]
    fn parse_uri(&self, uri: &str) -> McpResult<(String, i32)> {
        let without_scheme = uri.strip_prefix("content://").ok_or_else(|| {
            McpError::resource_not_found(
                "Invalid content URI: missing content:// scheme".to_string(),
            )
        })?;

        let parts: Vec<&str> = without_scheme.split('/').collect();
        if parts.len() != 2 {
            return Err(McpError::invalid_input(format!(
                "Invalid content URI format. Expected content://table/id, got {}",
                uri
            )));
        }

        let table = parts[0].to_string();
        let id = parts[1]
            .parse::<i32>()
            .map_err(|_| McpError::invalid_input(format!("Invalid ID in URI: {}", parts[1])))?;

        Ok((table, id))
    }

    /// Queries content from database.
    #[instrument(skip(self))]
    fn query_content(&self, table: &str, id: i32) -> McpResult<serde_json::Value> {
        let mut conn = establish_connection().map_err(|e| {
            McpError::execution_failed(format!("Database connection failed: {}", e))
        })?;

        get_content_by_id(&mut conn, table, id as i64)
            .map_err(|e| McpError::resource_not_found(format!("Content not found: {}", e)))
    }
}

impl Default for ContentResource {
    #[tracing::instrument]
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpResource for ContentResource {
    type Error = McpError;
    type ResourceInfo = ResourceInfo;

    #[tracing::instrument(skip(self))]
    fn uri_pattern(&self) -> &'static str {
        "content://"
    }

    #[tracing::instrument(skip(self))]
    fn description(&self) -> &'static str {
        "Access database content by table and ID"
    }

    #[instrument(skip(self), fields(uri))]
    async fn read(&self, uri: &str) -> Result<String, Self::Error> {
        let (table, id) = self.parse_uri(uri)?;
        debug!(table, id, "Reading content");

        let content = self.query_content(&table, id)?;

        // Format as JSON
        serde_json::to_string_pretty(&content)
            .map_err(|e| McpError::execution_failed(format!("Failed to serialize content: {}", e)))
    }

    #[instrument(skip(self))]
    async fn list(&self) -> Result<Vec<Self::ResourceInfo>, Self::Error> {
        let mut conn = establish_connection().map_err(|e| {
            McpError::execution_failed(format!("Database connection failed: {}", e))
        })?;

        // List recent content (limit 20 for performance)
        let rows = list_content(&mut conn, "content", None, 20)
            .map_err(|e| McpError::execution_failed(format!("Failed to list content: {}", e)))?;

        let resources = rows
            .into_iter()
            .filter_map(|row| {
                let id = row.get("id")?.as_i64()? as i32;
                let text = row.get("text_content")?.as_str()?;

                let preview = if text.len() > 50 {
                    format!("{}...", &text[..50])
                } else {
                    text.to_string()
                };

                Some(ResourceInfo::new(
                    format!("content://content/{}", id),
                    format!("Content {}", id),
                    preview,
                    Some("application/json".to_string()),
                ))
            })
            .collect();

        Ok(resources)
    }
}
