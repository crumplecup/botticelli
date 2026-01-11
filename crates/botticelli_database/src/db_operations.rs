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
    #[tracing::instrument(skip(pool))]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Execute a raw SQL query and return results as JSON.
    #[tracing::instrument(skip(conn), fields(query_len = query.len()))]
    fn execute_raw_query_sync(
        conn: &mut PgConnection,
        query: &str,
    ) -> BotticelliResult<Vec<Value>> {
        tracing::debug!("Executing raw SQL query");
        use diesel::sql_types::Jsonb;

        #[derive(QueryableByName)]
        struct JsonRow {
            #[diesel(sql_type = Jsonb)]
            data: Value,
        }

        // Wrap the query to return results as JSON
        let wrapped_query = format!("SELECT row_to_json(t)::jsonb as data FROM ({}) t", query);

        let results: Vec<JsonRow> = diesel::sql_query(&wrapped_query).load(conn).map_err(|e| {
            tracing::error!(error = %e, "Failed to execute query");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })?;

        let row_count = results.len();
        tracing::debug!(row_count, "Query executed successfully");
        Ok(results.into_iter().map(|r| r.data).collect())
    }

    /// List all tables in the database.
    #[tracing::instrument(skip(conn))]
    fn list_tables_sync(conn: &mut PgConnection) -> BotticelliResult<Vec<String>> {
        tracing::debug!("Listing database tables");
        #[derive(QueryableByName)]
        struct TableName {
            #[diesel(sql_type = diesel::sql_types::Text)]
            tablename: String,
        }

        let query =
            "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename";

        let tables: Vec<TableName> = diesel::sql_query(query).load(conn).map_err(|e| {
            tracing::error!(error = %e, "Failed to list tables");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })?;

        let table_count = tables.len();
        tracing::debug!(table_count, "Listed tables successfully");
        Ok(tables.into_iter().map(|t| t.tablename).collect())
    }
}

#[async_trait]
impl DatabaseRegistryOperations for DbOperationsImpl {
    type Error = botticelli_error::BotticelliError;

    #[tracing::instrument(skip(self), fields(query_len = query.len()))]
    async fn execute_query(&self, query: &str) -> BotticelliResult<Vec<Value>> {
        tracing::debug!("Getting connection for query execution");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!(error = %e, "Failed to get connection");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Connection(
                e.to_string(),
            ))
        })?;

        let results = Self::execute_raw_query_sync(&mut conn, query)?;
        tracing::info!(result_count = results.len(), "Query executed");
        Ok(results)
    }

    #[tracing::instrument(skip(self))]
    async fn list_tables(&self) -> BotticelliResult<Vec<String>> {
        tracing::debug!("Getting connection for table listing");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!(error = %e, "Failed to get connection");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Connection(
                e.to_string(),
            ))
        })?;

        let tables = Self::list_tables_sync(&mut conn)?;
        tracing::info!(table_count = tables.len(), "Listed tables");
        Ok(tables)
    }

    #[tracing::instrument(skip(self), fields(table))]
    async fn get_schema(&self, table: &str) -> BotticelliResult<Value> {
        tracing::debug!("Getting connection for schema reflection");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!(error = %e, "Failed to get connection");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Connection(
                e.to_string(),
            ))
        })?;

        tracing::debug!(table, "Reflecting table schema");
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

        tracing::info!(
            table,
            column_count = schema.columns.len(),
            "Retrieved schema"
        );
        Ok(schema_json)
    }

    #[tracing::instrument(skip(self), fields(table_name, status_filter = ?status_filter, limit))]
    async fn query_content(
        &self,
        table_name: &str,
        status_filter: Option<&str>,
        limit: i64,
    ) -> BotticelliResult<Vec<Value>> {
        tracing::debug!("Getting connection for content query");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!(error = %e, "Failed to get connection");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Connection(
                e.to_string(),
            ))
        })?;

        tracing::debug!(table_name, status_filter, limit, "Querying content");
        let rows = crate::list_content(&mut conn, table_name, status_filter, limit as usize)?;

        let results = rows
            .into_iter()
            .map(|row| {
                serde_json::to_value(&row).map_err(|e| {
                    tracing::error!(error = %e, "Failed to serialize row");
                    botticelli_error::DatabaseError::new(
                        botticelli_error::DatabaseErrorKind::Serialization(e.to_string()),
                    )
                    .into()
                })
            })
            .collect::<BotticelliResult<Vec<_>>>()?;

        tracing::info!(table_name, row_count = results.len(), "Queried content");
        Ok(results)
    }

    #[tracing::instrument(skip(self), fields(table_name, has_narrative = narrative_file.is_some(), has_description = description.is_some()))]
    async fn create_table(
        &self,
        table_name: &str,
        template_source: &str,
        narrative_file: Option<&str>,
        description: Option<&str>,
    ) -> BotticelliResult<()> {
        tracing::debug!("Getting connection for table creation");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!(error = %e, "Failed to get connection");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Connection(
                e.to_string(),
            ))
        })?;

        tracing::debug!(table_name, "Creating content table");
        crate::create_content_table(
            &mut conn,
            table_name,
            template_source,
            narrative_file,
            description,
        )?;

        tracing::info!(table_name, "Created table");
        Ok(())
    }

    #[tracing::instrument(skip(self), fields(table_name))]
    async fn table_exists(&self, table_name: &str) -> BotticelliResult<bool> {
        tracing::debug!("Getting connection for table existence check");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!(error = %e, "Failed to get connection");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Connection(
                e.to_string(),
            ))
        })?;

        let exists = crate::table_exists(&mut conn, table_name)?;
        tracing::debug!(table_name, exists, "Checked table existence");
        Ok(exists)
    }
}
