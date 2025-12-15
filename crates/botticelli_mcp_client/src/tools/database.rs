use async_trait::async_trait;
#[cfg(feature = "database")]
use botticelli_database::{
    DbPool, create_content_table, list_content, reflect_table_schema, table_exists,
};
use botticelli_error::DatabaseError;
use serde_json::{Value, json};

use crate::NarrativeTool;

/// Tool for creating database tables.
///
/// Available with the `database` feature.
#[cfg(feature = "database")]
#[derive(Debug, Clone, derive_new::new)]
pub struct CreateTableTool {
    pool: DbPool,
}

#[async_trait]
impl NarrativeTool for CreateTableTool {
    fn name(&self) -> &str {
        "create_table"
    }

    fn description(&self) -> &str {
        "Create a new database table with specified schema"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "table_name": {
                    "type": "string",
                    "description": "Name of the table to create"
                },
                "columns": {
                    "type": "array",
                    "description": "Column definitions",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "data_type": {"type": "string"},
                            "nullable": {"type": "boolean"}
                        },
                        "required": ["name", "data_type"]
                    }
                }
            },
            "required": ["table_name", "columns"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let table_name = input["table_name"]
            .as_str()
            .ok_or("Missing table_name")?;
        
        let mut conn = self.pool.get()
            .map_err(|e| DatabaseError::connection_error(e.to_string()))?;

        create_content_table(&mut conn, table_name)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(json!({
            "success": true,
            "table_name": table_name,
            "message": format!("Table '{}' created successfully", table_name)
        }))
    }
}

/// Tool for querying database tables.
///
/// Available with the `database` feature.
#[cfg(feature = "database")]
#[derive(Debug, Clone, derive_new::new)]
pub struct QueryTableTool {
    pool: DbPool,
}

#[async_trait]
impl NarrativeTool for QueryTableTool {
    fn name(&self) -> &str {
        "query_table"
    }

    fn description(&self) -> &str {
        "Query a database table and return results"
    }

    fn input_schema(&self) -> Value {
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
        })
    }

    async fn execute(&self, input: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let table_name = input["table_name"]
            .as_str()
            .ok_or("Missing table_name")?;
        let limit = input["limit"].as_i64().unwrap_or(100);

        let mut conn = self.pool.get()
            .map_err(|e| DatabaseError::connection_error(e.to_string()))?;

        let rows = list_content(&mut conn, table_name, limit)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(json!({
            "success": true,
            "table_name": table_name,
            "row_count": rows.len(),
            "rows": rows
        }))
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
impl NarrativeTool for InspectTableTool {
    fn name(&self) -> &str {
        "inspect_table"
    }

    fn description(&self) -> &str {
        "Inspect the schema of a database table"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "table_name": {
                    "type": "string",
                    "description": "Name of the table to inspect"
                }
            },
            "required": ["table_name"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let table_name = input["table_name"]
            .as_str()
            .ok_or("Missing table_name")?;

        let mut conn = self.pool.get()
            .map_err(|e| DatabaseError::connection_error(e.to_string()))?;

        let schema = reflect_table_schema(&mut conn, table_name)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(json!({
            "success": true,
            "table_name": table_name,
            "schema": {
                "table_name": schema.table_name(),
                "columns": schema.columns().iter().map(|col| {
                    json!({
                        "name": col.name(),
                        "data_type": col.data_type(),
                        "is_nullable": col.is_nullable(),
                        "is_primary_key": col.is_primary_key()
                    })
                }).collect::<Vec<_>>()
            }
        }))
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
impl NarrativeTool for TableExistsTool {
    fn name(&self) -> &str {
        "table_exists"
    }

    fn description(&self) -> &str {
        "Check if a database table exists"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "table_name": {
                    "type": "string",
                    "description": "Name of the table to check"
                }
            },
            "required": ["table_name"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let table_name = input["table_name"]
            .as_str()
            .ok_or("Missing table_name")?;

        let mut conn = self.pool.get()
            .map_err(|e| DatabaseError::connection_error(e.to_string()))?;

        let exists = table_exists(&mut conn, table_name)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(json!({
            "exists": exists,
            "table_name": table_name
        }))
    }
}
