//! Table query registry trait.

use async_trait::async_trait;

/// Trait for querying database tables in narratives.
#[async_trait]
pub trait TableQueryRegistry: Send + Sync {
    /// Error type for table operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Query a database table and return results in the specified format.
    async fn query_table(&self, query: &dyn crate::TableView) -> Result<String, Self::Error>;

    /// Query a table and atomically delete the returned rows (destructive read).
    async fn query_and_delete_table(
        &self,
        query: &dyn crate::TableView,
    ) -> Result<String, Self::Error>;
}
