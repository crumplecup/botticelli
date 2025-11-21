//! Backend trait for TUI data operations.
//!
//! This module defines the backend trait that allows the TUI to work with
//! different data sources (database, mock data, etc.) without coupling to
//! specific implementations.

use crate::{ContentRow, TuiResult};

/// Backend trait for TUI data operations.
///
/// Implementations provide data access for the TUI without exposing
/// implementation details. This allows the TUI to work with databases,
/// mock data, or other sources.
///
/// Note: Only requires `Send` (not `Sync`) since the TUI is single-threaded.
/// Database connections are not thread-safe and don't need to be.
pub trait TuiBackend: Send {
    /// List content items from the data source.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table/collection to query
    /// * `limit` - Maximum number of items to return
    ///
    /// # Returns
    ///
    /// Vector of content rows
    fn list_content(&mut self, table_name: &str, limit: i64) -> TuiResult<Vec<ContentRow>>;

    /// Update metadata for a content item.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table/collection
    /// * `id` - Item ID to update
    /// * `tags` - New tags
    /// * `rating` - New rating (1-5)
    /// * `status` - New review status
    fn update_metadata(
        &mut self,
        table_name: &str,
        id: i64,
        tags: &[String],
        rating: Option<i32>,
        status: &str,
    ) -> TuiResult<()>;

    /// Delete a content item.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table/collection
    /// * `id` - Item ID to delete
    fn delete_item(&mut self, table_name: &str, id: i64) -> TuiResult<()>;

    /// Export content items to JSON.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table/collection
    /// * `ids` - Item IDs to export
    ///
    /// # Returns
    ///
    /// JSON string containing exported items
    fn export_items(&mut self, table_name: &str, ids: &[i64]) -> TuiResult<String>;
}
