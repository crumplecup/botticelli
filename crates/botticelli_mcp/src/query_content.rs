//! Query content tool types for database queries.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;

/// Parameters for querying content from database tables.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct QueryContentParams {
    /// The table name to query (e.g., 'blog_posts', 'tweets')
    table: String,

    /// Maximum number of results to return (default: 10, max: 100)
    #[serde(default = "default_limit")]
    limit: i64,
}

impl QueryContentParams {
    /// Create new query content parameters.
    #[instrument]
    pub fn new(table: String, limit: i64) -> Self {
        Self { table, limit }
    }
}

#[tracing::instrument]
fn default_limit() -> i64 {
    10
}

/// Result of a content query operation.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct QueryContentResult {
    /// Status of the operation
    status: String,

    /// Table that was queried
    table: String,

    /// Number of rows returned
    count: usize,

    /// Limit applied to the query
    limit: i64,

    /// Rows returned from the query
    rows: Vec<Value>,
}

impl QueryContentResult {
    /// Create a new query content result.
    ///
    /// # Arguments
    ///
    /// * `table` - Table name that was queried
    /// * `limit` - Limit applied to the query
    /// * `rows` - Rows returned from the query
    ///
    /// # Returns
    ///
    /// A new `QueryContentResult` with success status.
    #[instrument(skip(rows))]
    pub fn new(table: String, limit: i64, rows: Vec<Value>) -> Self {
        let count = rows.len();
        Self {
            status: "success".to_string(),
            table,
            count,
            limit,
            rows,
        }
    }
}
