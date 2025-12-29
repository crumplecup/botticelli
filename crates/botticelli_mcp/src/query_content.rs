//! Query content tool types for database queries.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for querying content from database tables.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryContentParams {
    /// The table name to query (e.g., 'blog_posts', 'tweets')
    pub table: String,

    /// Maximum number of results to return (default: 10, max: 100)
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    10
}

/// Result of a content query operation.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct QueryContentResult {
    /// Status of the operation
    pub status: String,

    /// Table that was queried
    pub table: String,

    /// Number of rows returned
    pub count: usize,

    /// Limit applied to the query
    pub limit: i64,

    /// Rows returned from the query
    pub rows: Vec<Value>,
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
