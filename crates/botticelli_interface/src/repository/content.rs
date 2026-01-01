//! Content repository trait.

use async_trait::async_trait;

/// Trait for managing generated content in database tables.
#[async_trait]
pub trait ContentRepository: Send + Sync {
    /// Error type for content operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new content table with the specified schema.
    async fn create_content_table(
        &self,
        table_name: &str,
        schema: &serde_json::Value,
    ) -> Result<String, Self::Error>;

    /// Insert generated content into a table.
    async fn insert_content(
        &self,
        table_name: &str,
        content: &serde_json::Value,
    ) -> Result<i32, Self::Error>;

    /// Query content from a table with optional filtering.
    async fn query_content(
        &self,
        table_name: &str,
        filter: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, Self::Error>;
}
