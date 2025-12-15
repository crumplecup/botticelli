//! Implementation of DatabaseRegistryOperations trait.

use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use botticelli_interface::DatabaseRegistryOperations;
use serde_json::Value;

use crate::{DbPool, reflect_table_schema};

/// Database operations implementation wrapping DbPool.
#[derive(Debug, Clone)]
pub struct DbOperationsImpl {
    pool: DbPool,
}

impl DbOperationsImpl {
    /// Create new database operations with connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DatabaseRegistryOperations for DbOperationsImpl {
    async fn execute_query(&self, query: &str) -> BotticelliResult<Vec<Value>> {
        // TODO: Implement raw query execution
        tracing::warn!(query = %query, "Raw query execution not yet implemented");
        Ok(vec![])
    }

    async fn list_tables(&self) -> BotticelliResult<Vec<String>> {
        // TODO: Implement table listing
        tracing::warn!("Table listing not yet implemented");
        Ok(vec![])
    }

    async fn get_schema(&self, table: &str) -> BotticelliResult<Value> {
        let mut conn = self.pool.get()
            .map_err(|e| botticelli_error::DatabaseError::new(
                botticelli_error::DatabaseErrorKind::Connection(e.to_string())
            ))?;
        
        let schema = reflect_table_schema(&mut conn, table)?;
        
        // Manually construct JSON since TableSchema doesn't derive Serialize
        let schema_json = serde_json::json!({
            "table_name": schema.table_name,
            "columns": schema.columns.iter().map(|col| {
                serde_json::json!({
                    "name": col.name,
                    "data_type": col.data_type,
                    "is_nullable": col.is_nullable,
                })
            }).collect::<Vec<_>>()
        });
        
        Ok(schema_json)
    }

    async fn query_content(
        &self,
        table_name: &str,
        status_filter: Option<&str>,
        limit: i64,
    ) -> BotticelliResult<Vec<Value>> {
        let mut conn = self.pool.get()
            .map_err(|e| botticelli_error::DatabaseError::new(
                botticelli_error::DatabaseErrorKind::Connection(e.to_string())
            ))?;
        
        let rows = crate::list_content(&mut conn, table_name, status_filter, limit as usize)?;
        
        rows.into_iter()
            .map(|row| serde_json::to_value(&row)
                .map_err(|e| botticelli_error::DatabaseError::new(
                    botticelli_error::DatabaseErrorKind::Serialization(e.to_string())
                ).into()))
            .collect()
    }
}
