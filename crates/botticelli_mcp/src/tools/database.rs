//! Database query tools for MCP server.

use crate::tools::McpTool;
use botticelli_error::{McpError, McpResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::{debug, instrument};

#[cfg(feature = "database")]
use botticelli_database::{establish_connection, list_content};

/// Tool for querying content from database tables.
pub struct QueryContentTool;

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

    #[instrument(skip(self, input), fields(table, limit))]
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

        debug!(table = %table, limit, "Querying content");

        #[cfg(feature = "database")]
        {
            // Query the database
            let mut conn = establish_connection().map_err(|e| {
                McpError::execution_failed(format!("Database connection failed: {}", e))
            })?;

            let rows = list_content(&mut conn, table, None, limit as usize)
                .map_err(|e| McpError::execution_failed(format!("Query failed: {}", e)))?;

            debug!(count = rows.len(), "Retrieved rows from database");

            Ok(json!({
                "status": "success",
                "table": table,
                "count": rows.len(),
                "limit": limit,
                "rows": rows
            }))
        }

        #[cfg(not(feature = "database"))]
        {
            Ok(json!({
                "status": "not_available",
                "message": "Database feature not enabled. Build with --features database",
                "requested": {
                    "table": table,
                    "limit": limit
                }
            }))
        }
    }
}
