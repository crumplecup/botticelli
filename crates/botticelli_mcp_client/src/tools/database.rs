use async_trait::async_trait;
use botticelli_interface::DatabaseRegistryOperations;
use pmcp::{Content, ToolInfo};
use serde_json::{Value, json};

use crate::{McpClientError, McpClientErrorKind, ToolHandler};

/// Tool for creating database tables.
///
/// Available with the `database` feature.
#[cfg(feature = "database")]
#[derive(Debug, Clone)]
pub struct CreateTableTool<D: DatabaseRegistryOperations> {
    db_ops: D,
}

#[cfg(feature = "database")]
impl<D: DatabaseRegistryOperations> CreateTableTool<D> {
    /// Create a new CreateTableTool with the given database operations.
    pub fn new(db_ops: D) -> Self {
        Self { db_ops }
    }
}

#[cfg(feature = "database")]
#[async_trait]
impl<D: DatabaseRegistryOperations + Send + Sync> ToolHandler for CreateTableTool<D> {
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
        let table_name = input["table_name"].as_str().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Missing table_name".to_string(),
            ))
        })?;

        let template_source = input["template_source"].as_str().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Missing template_source".to_string(),
            ))
        })?;

        let narrative_file = input["narrative_file"].as_str();
        let description = input["description"].as_str();

        self.db_ops
            .create_table(table_name, template_source, narrative_file, description)
            .await
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!(
                    "Table creation error: {}",
                    e
                )))
            })?;

        let result = json!({
            "success": true,
            "table_name": table_name,
            "message": format!("Table '{}' created successfully", table_name)
        });

        Ok(vec![Content::Text {
            text: result.to_string(),
        }])
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
    /// Create a new QueryTableTool with the given database operations.
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
        let table_name = input["table_name"].as_str().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Missing table_name".to_string(),
            ))
        })?;
        let limit = input["limit"].as_i64().unwrap_or(100);
        let status_filter = input["status_filter"].as_str();

        let rows = self
            .db_ops
            .query_content(table_name, status_filter, limit)
            .await
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!(
                    "Query error: {}",
                    e
                )))
            })?;

        let result = json!({
            "success": true,
            "table_name": table_name,
            "row_count": rows.len(),
            "rows": rows
        });

        Ok(vec![Content::Text {
            text: result.to_string(),
        }])
    }
}

/// Tool for inspecting table schema.
///
/// Available with the `database` feature.
#[cfg(feature = "database")]
#[derive(Debug, Clone)]
pub struct InspectTableTool<D: DatabaseRegistryOperations> {
    db_ops: D,
}

#[cfg(feature = "database")]
impl<D: DatabaseRegistryOperations> InspectTableTool<D> {
    /// Create a new InspectTableTool with the given database operations.
    pub fn new(db_ops: D) -> Self {
        Self { db_ops }
    }
}

#[cfg(feature = "database")]
#[async_trait]
impl<D: DatabaseRegistryOperations + Send + Sync> ToolHandler for InspectTableTool<D> {
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
        let table_name = input["table_name"].as_str().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Missing table_name".to_string(),
            ))
        })?;

        let schema = self.db_ops.get_schema(table_name).await.map_err(|e| {
            McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!(
                "Schema reflection error: {}",
                e
            )))
        })?;

        let result = json!({
            "success": true,
            "table_name": table_name,
            "schema": schema
        });

        Ok(vec![Content::Text {
            text: result.to_string(),
        }])
    }
}

/// Tool for checking if a table exists.
///
/// Available with the `database` feature.
#[cfg(feature = "database")]
#[derive(Debug, Clone)]
pub struct TableExistsTool<D: DatabaseRegistryOperations> {
    db_ops: D,
}

#[cfg(feature = "database")]
impl<D: DatabaseRegistryOperations> TableExistsTool<D> {
    /// Create a new TableExistsTool with the given database operations.
    pub fn new(db_ops: D) -> Self {
        Self { db_ops }
    }
}

#[async_trait]
impl<D: DatabaseRegistryOperations + Send + Sync> ToolHandler for TableExistsTool<D> {
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
        let table_name = input["table_name"].as_str().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Missing table_name".to_string(),
            ))
        })?;

        let exists = self.db_ops.table_exists(table_name).await.map_err(|e| {
            McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!(
                "Table check error: {}",
                e
            )))
        })?;

        let result = json!({
            "exists": exists,
            "table_name": table_name
        });

        Ok(vec![Content::Text {
            text: result.to_string(),
        }])
    }
}
