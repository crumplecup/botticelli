//! Registry storage trait for elicitation data types.

use async_trait::async_trait;
use botticelli_error::McpResult;

/// Generic registry storage operations for CRUD on data types.
#[async_trait]
pub trait RegistryStorage<T> {
    /// Create a new entry
    async fn create(&self, item: T) -> McpResult<String>;

    /// Read an entry by ID
    async fn read(&self, id: &str) -> McpResult<T>;

    /// Update an existing entry
    async fn update(&self, id: &str, item: T) -> McpResult<()>;

    /// Delete an entry
    async fn delete(&self, id: &str) -> McpResult<()>;

    /// List all entries
    async fn list(&self) -> McpResult<Vec<T>>;
}
