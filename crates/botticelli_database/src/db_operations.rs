//! Implementation of DatabaseRegistryOperations trait.

use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use botticelli_interface::DatabaseRegistryOperations;
use diesel::prelude::*;
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
    
    /// Execute a raw SQL query and return results as JSON.
    fn execute_raw_query_sync(conn: &mut PgConnection, query: &str) -> BotticelliResult<Vec<Value>> {
        use diesel::sql_types::Jsonb;
        
        #[derive(QueryableByName)]
        struct JsonRow {
            #[diesel(sql_type = Jsonb)]
            data: Value,
        }
        
        // Wrap the query to return results as JSON
        let wrapped_query = format!("SELECT row_to_json(t)::jsonb as data FROM ({}) t", query);
        
        let results: Vec<JsonRow> = diesel::sql_query(&wrapped_query)
            .load(conn)
            .map_err(|e| botticelli_error::DatabaseError::new(
                botticelli_error::DatabaseErrorKind::Query(e.to_string())
            ))?;
        
        Ok(results.into_iter().map(|r| r.data).collect())
    }
    
    /// List all tables in the database.
    fn list_tables_sync(conn: &mut PgConnection) -> BotticelliResult<Vec<String>> {
        #[derive(QueryableByName)]
        struct TableName {
            #[diesel(sql_type = diesel::sql_types::Text)]
            tablename: String,
        }
        
        let query = "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename";
        
        let tables: Vec<TableName> = diesel::sql_query(query)
            .load(conn)
            .map_err(|e| botticelli_error::DatabaseError::new(
                botticelli_error::DatabaseErrorKind::Query(e.to_string())
            ))?;
        
        Ok(tables.into_iter().map(|t| t.tablename).collect())
    }
}

#[async_trait]
impl DatabaseRegistryOperations for DbOperationsImpl {
    async fn execute_query(&self, query: &str) -> BotticelliResult<Vec<Value>> {
        let mut conn = self.pool.get()
            .map_err(|e| botticelli_error::DatabaseError::new(
                botticelli_error::DatabaseErrorKind::Connection(e.to_string())
            ))?;
        
        Self::execute_raw_query_sync(&mut conn, query).map_err(Into::into)
    }

    async fn list_tables(&self) -> BotticelliResult<Vec<String>> {
        let mut conn = self.pool.get()
            .map_err(|e| botticelli_error::DatabaseError::new(
                botticelli_error::DatabaseErrorKind::Connection(e.to_string())
            ))?;
        
        Self::list_tables_sync(&mut conn).map_err(Into::into)
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

    async fn create_table(
        &self,
        table_name: &str,
        template_source: &str,
        narrative_file: Option<&str>,
        description: Option<&str>,
    ) -> BotticelliResult<()> {
        let mut conn = self.pool.get()
            .map_err(|e| botticelli_error::DatabaseError::new(
                botticelli_error::DatabaseErrorKind::Connection(e.to_string())
            ))?;
        
        crate::create_content_table(&mut conn, table_name, template_source, narrative_file, description)
            .map_err(Into::into)
    }

    async fn table_exists(&self, table_name: &str) -> BotticelliResult<bool> {
        let mut conn = self.pool.get()
            .map_err(|e| botticelli_error::DatabaseError::new(
                botticelli_error::DatabaseErrorKind::Connection(e.to_string())
            ))?;
        
        crate::table_exists(&mut conn, table_name).map_err(Into::into)
    }
}
