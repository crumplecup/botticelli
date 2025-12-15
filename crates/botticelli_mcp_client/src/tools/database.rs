use async_trait::async_trait;
#[cfg(feature = "database")]
use botticelli_database::{
    create_content_table, list_content, reflect_table_schema, table_exists, DbPool,
};
use botticelli_interface::DatabaseRegistryOperations;
use pmcp::{Content, ToolInfo};
use serde_json::{json, Value};

use crate::{McpClientError, McpClientErrorKind, ToolHandler};

/// Tool for creating database tables.
///
/// Available with the `database` feature.
#[cfg(feature = "database")]
#[derive(Debug, Clone, derive_new::new)]
pub struct CreateTableTool {
    pool: DbPool,
}

#[async_trait]
impl ToolHandler for CreateTableTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "create_table",
            Some("Create a new database table with specified schema".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "table_name": {
                        "type": "string",
                        "description": "Name of the table to create"
                    },
                    "template_source": {
                        "type": "string",
                        "description": "Source template for the table"
                    },
                    "narrative_file": {
                        "type": "string",
                        "description": "Optional narrative file path"
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional table description"
                    }
                },
                "required": ["table_name", "template_source"]
            }),
        )
    }

    async fn execute(&self, input: Value) -> Result<Vec<Content>, McpClientError> {
        let table_name = input["table_name"]
            .as_str()
            .ok_or_else(|| McpClientError::new(McpClientErrorKind::InvalidToolCall("Missing table_name".to_string())))?;
        
        let template_source = input["template_source"]
            .as_str()
            .ok_or_else(|| McpClientError::new(McpClientErrorKind::InvalidToolCall("Missing template_source".to_string())))?;
        
        let narrative_file = input["narrative_file"].as_str();
        let description = input["description"].as_str();
        
        let mut conn = self.pool.get()
            .map_err(|e| McpClientError::new(McpClientErrorKind::ConnectionError(format!("Connection error: {}", e))))?;

        create_content_table(&mut conn, table_name, template_source, narrative_file, description)
            .map_err(|e| McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!("Table creation error: {}", e))))?;

        let result = json!({
            "success": true,
            "table_name": table_name,
            "message": format!("Table '{}' created successfully", table_name)
        });

        Ok(vec![Content::Text { text: result.to_string() }])
    }
}

/// Tool for querying database tables.
///
/// Available with the `database` feature.
#[cfg(feature = "database")]
#[derive(Debug, Clone)]
pub struct QueryTableTool<D: DatabaseRegistryOperations> {
    db_ops: D,
}

#[cfg(feature = "database")]
impl<D: DatabaseRegistryOperations> QueryTableTool<D> {
    pub fn new(db_ops: D) -> Self {
        Self { db_ops }
    }
}

#[cfg(feature = "database")]
#[async_trait]
impl<D: DatabaseRegistryOperations + Send + Sync> ToolHandler for QueryTableTool<D> {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "query_table",
            Some("Query rows from a database table".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "table_name": {
                        "type": "string",
                        "description": "Name of the table to query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of rows to return",
                        "default": 100
                    }
                },
                "required": ["table_name"]
            }),
        )
    }

    async fn execute(&self, input: Value) -> Result<Vec<Content>, McpClientError> {
        let table_name = input["table_name"]
            .as_str()
            .ok_or_else(|| McpClientError::new(McpClientErrorKind::InvalidToolCall("Missing table_name".to_string())))?;
        let limit = input["limit"].as_i64().unwrap_or(100);
        let status_filter = input["status_filter"].as_str();

        let rows = self.db_ops.query_content(table_name, status_filter, limit)
            .await
            .map_err(|e| McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!("Query error: {}", e))))?;

        let result = json!({
            "success": true,
            "table_name": table_name,
            "row_count": rows.len(),
            "rows": rows
        });

        Ok(vec![Content::Text { text: result.to_string() }])
    }
}

/// Tool for inspecting table schema.
///
/// Available with the `database` feature.
#[cfg(feature = "database")]
#[derive(Debug, Clone, derive_new::new)]
pub struct InspectTableTool {
    pool: DbPool,
}

#[async_trait]
impl ToolHandler for InspectTableTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "inspect_table",
            Some("Inspect the schema of a database table".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "table_name": {
                        "type": "string",
                        "description": "Name of the table to inspect"
                    }
                },
                "required": ["table_name"]
            }),
        )
    }

    async fn execute(&self, input: Value) -> Result<Vec<Content>, McpClientError> {
        let table_name = input["table_name"]
            .as_str()
            .ok_or_else(|| McpClientError::new(McpClientErrorKind::InvalidToolCall("Missing table_name".to_string())))?;

        let mut conn = self.pool.get()
            .map_err(|e| McpClientError::new(McpClientErrorKind::ConnectionError(format!("Connection error: {}", e))))?;

        let schema = reflect_table_schema(&mut conn, table_name)
            .map_err(|e| McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!("Schema reflection error: {}", e))))?;

        let result = json!({
            "success": true,
            "table_name": table_name,
            "schema": {
                "table_name": schema.table_name,
                "columns": schema.columns.iter().map(|col| {
                    json!({
                        "name": col.name,
                        "data_type": col.data_type,
                        "is_nullable": col.is_nullable
                    })
                }).collect::<Vec<_>>()
            }
        });

        Ok(vec![Content::Text { text: result.to_string() }])
    }
}

/// Tool for checking if a table exists.
///
/// Available with the `database` feature.
#[cfg(feature = "database")]
#[derive(Debug, Clone, derive_new::new)]
pub struct TableExistsTool {
    pool: DbPool,
}

#[async_trait]
impl ToolHandler for TableExistsTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "table_exists",
            Some("Check if a database table exists".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "table_name": {
                        "type": "string",
                        "description": "Name of the table to check"
                    }
                },
                "required": ["table_name"]
            }),
        )
    }

    async fn execute(&self, input: Value) -> Result<Vec<Content>, McpClientError> {
        let table_name = input["table_name"]
            .as_str()
            .ok_or_else(|| McpClientError::new(McpClientErrorKind::InvalidToolCall("Missing table_name".to_string())))?;

        let mut conn = self.pool.get()
            .map_err(|e| McpClientError::new(McpClientErrorKind::ConnectionError(format!("Connection error: {}", e))))?;

        let exists = table_exists(&mut conn, table_name)
            .map_err(|e| McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!("Table check error: {}", e))))?;

        let result = json!({
            "exists": exists,
            "table_name": table_name
        });

        Ok(vec![Content::Text { text: result.to_string() }])
    }
}
