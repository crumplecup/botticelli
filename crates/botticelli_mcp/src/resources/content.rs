//! Content resource for database content.

use async_trait::async_trait;
use crate::ResourceInfo;
use botticelli_database::{establish_connection, get_content_by_id, list_content};
use botticelli_error::BotticelliResult;
use botticelli_interface::McpResource;
use rmcp::handler::server::wrapper::Parameters;
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
    fn parse_uri(&self, uri: &str) -> BotticelliResult<(String, i32)> {
        let without_scheme = uri.strip_prefix("content://").ok_or_else(|| {
            botticelli_error::IoError::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid content URI: missing content:// scheme",
            ))
        })?;

        let parts: Vec<&str> = without_scheme.split('/').collect();
        if parts.len() != 2 {
            return Err(botticelli_error::IoError::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid content URI format. Expected content://table/id, got {}",
                    uri
                ),
            ))
            .into());
        }

        let table = parts[0].to_string();
        let id = parts[1].parse::<i32>().map_err(|e| {
            botticelli_error::IoError::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid ID in URI '{}': {}", parts[1], e),
            ))
        })?;

        Ok((table, id))
    }

    /// Queries content from database.
    #[instrument(skip(self))]
    fn query_content(&self, table: &str, id: i32) -> BotticelliResult<serde_json::Value> {
        let mut conn = establish_connection()?;
        get_content_by_id(&mut conn, table, id as i64)
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
    type Error = botticelli_error::BotticelliError;
    type ResourceInfo = ResourceInfo;

    #[tracing::instrument(skip(self))]
    fn uri_pattern(&self) -> &'static str {
        "content://"
    }

    #[tracing::instrument(skip(self))]
    fn description(&self) -> &'static str {
        "Access database content by table and ID"
    }

    #[instrument(skip(self), fields(uri = %params.0.uri))]
    async fn read(
        &self,
        params: Parameters<botticelli_interface::ReadParams>,
    ) -> Result<rmcp::Json<botticelli_interface::ReadResult>, rmcp::ErrorData> {
        let (table, id) = self.parse_uri(&params.0.uri)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
        
        debug!(table, id, "Reading content");

        let content = self.query_content(&table, id)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        // Format as JSON
        let content_str = serde_json::to_string_pretty(&content)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
        
        Ok(rmcp::Json(botticelli_interface::ReadResult {
            content: content_str,
        }))
    }

    #[instrument(skip(self))]
    async fn list(&self) -> Result<Vec<Self::ResourceInfo>, Self::Error> {
        let mut conn = establish_connection()?;

        // List recent content (limit 20 for performance)
        let rows = list_content(&mut conn, "content", None, 20)?;

        let resources = rows
            .into_iter()
            .filter_map(|row: serde_json::Value| {
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
