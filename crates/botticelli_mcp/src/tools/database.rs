//! Database query tools for MCP server.

use crate::tools::McpTool;
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use botticelli_interface::DatabaseRegistryOperations;
use serde_json::{json, Value};
use std::sync::Arc;

/// Tool for querying content from database tables.
pub struct QueryContentTool {
    db_ops: Arc<dyn DatabaseRegistryOperations>,
}

impl QueryContentTool {
    /// Create a new query content tool with database operations.
    pub fn new(db_ops: Arc<dyn DatabaseRegistryOperations>) -> Self {
        Self { db_ops }
    }
}

#[async_trait]
impl McpTool for QueryContentTool {
    fn name(&self) -> &str {
        "query_content"
    }

    fn description(&self) -> &str {
        "Query content from database tables. Returns a list of content items with their metadata."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "table": {
                    "type": "string",
                    "description": "The table name to query (e.g., 'blog_posts', 'tweets')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 10, max: 100)",
                    "default": 10,
                    "minimum": 1,
                    "maximum": 100
                }
            },
            "required": ["table"]
        })
    }

    #[tracing::instrument(skip(self, input), fields(table, limit))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let table = input
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'table' field".to_string()))?;

        let limit = input
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(10)
            .clamp(1, 100);

        tracing::debug!(table = %table, limit, "Querying content");

        // Execute raw query using trait
        let query = format!("SELECT * FROM {} LIMIT {}", table, limit);
        let rows = self
            .db_ops
            .execute_query(&query)
            .await
            .map_err(|e| McpError::execution_failed(format!("Query failed: {}", e)))?;

        tracing::debug!(count = rows.len(), "Retrieved rows from database");

        Ok(json!({
            "status": "success",
            "table": table,
            "count": rows.len(),
            "limit": limit,
            "rows": rows
        }))
    }
}
