//! Knowledge table abstraction for actor data access.

use crate::{ActorError, ActorErrorKind, ActorResult};
use diesel::PgConnection;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

/// Wrapper for knowledge table access.
///
/// Provides type-safe access to database tables produced by narratives.
/// Knowledge tables contain structured data that actors consume.
#[derive(Debug, Clone)]
pub struct KnowledgeTable {
    name: String,
}

impl KnowledgeTable {
    /// Create a new knowledge table reference.
    ///
    /// # Arguments
    ///
    /// * `name` - Table name
    #[tracing::instrument(skip_all, fields(table_name))]
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        tracing::debug!(table_name = %name, "Creating knowledge table reference");
        Self { name }
    }

    /// Get the table name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Query all rows from the knowledge table.
    ///
    /// # Arguments
    ///
    /// * `conn` - Database connection
    ///
    /// # Returns
    ///
    /// Vector of rows as JSON objects.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Table does not exist
    /// - Query fails
    /// - JSON parsing fails
    #[tracing::instrument(skip(self, conn), fields(table_name = %self.name))]
    pub fn query(&self, conn: &mut PgConnection) -> ActorResult<Vec<JsonValue>> {
        tracing::debug!("Querying knowledge table");

        // Use raw SQL to query dynamic table names
        let query = format!("SELECT row_to_json(t.*) as data FROM {} as t", self.name);

        tracing::debug!(sql = %query, "Executing query");

        let results: Vec<JsonValue> = diesel::sql_query(&query)
            .load::<QueryRow>(conn)
            .map_err(|e| {
                tracing::error!(error = ?e, "Knowledge table query failed");
                ActorError::new(ActorErrorKind::KnowledgeTableNotFound(format!(
                    "{}: {}",
                    self.name, e
                )))
            })?
            .into_iter()
            .map(|row| row.data)
            .collect();

        tracing::info!(count = results.len(), "Retrieved rows from knowledge table");
        Ok(results)
    }

    /// Query rows with a WHERE clause.
    ///
    /// # Arguments
    ///
    /// * `conn` - Database connection
    /// * `where_clause` - SQL WHERE clause (without "WHERE" keyword)
    ///
    /// # Returns
    ///
    /// Vector of matching rows as JSON objects.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Table does not exist
    /// - Query fails
    /// - Invalid WHERE clause
    #[tracing::instrument(skip(self, conn), fields(table_name = %self.name, where_clause))]
    pub fn query_where(
        &self,
        conn: &mut PgConnection,
        where_clause: &str,
    ) -> ActorResult<Vec<JsonValue>> {
        tracing::debug!("Querying knowledge table with WHERE clause");

        let query = format!(
            "SELECT row_to_json(t.*) as data FROM {} as t WHERE {}",
            self.name, where_clause
        );

        tracing::debug!(sql = %query, "Executing query");

        let results: Vec<JsonValue> = diesel::sql_query(&query)
            .load::<QueryRow>(conn)
            .map_err(|e| {
                tracing::error!(error = ?e, "Knowledge table query failed");
                ActorError::new(ActorErrorKind::KnowledgeTableNotFound(format!(
                    "{}: {}",
                    self.name, e
                )))
            })?
            .into_iter()
            .map(|row| row.data)
            .collect();

        tracing::info!(
            count = results.len(),
            "Retrieved filtered rows from knowledge table"
        );
        Ok(results)
    }

    /// Get row count from table.
    ///
    /// # Arguments
    ///
    /// * `conn` - Database connection
    ///
    /// # Returns
    ///
    /// Number of rows in table.
    ///
    /// # Errors
    ///
    /// Returns error if table does not exist or query fails.
    #[tracing::instrument(skip(self, conn), fields(table_name = %self.name))]
    pub fn count(&self, conn: &mut PgConnection) -> ActorResult<i64> {
        tracing::debug!("Counting rows in knowledge table");

        let query = format!("SELECT COUNT(*) as count FROM {}", self.name);

        tracing::debug!(sql = %query, "Executing query");

        let result: CountRow = diesel::sql_query(&query).get_result(conn).map_err(|e| {
            tracing::error!(error = ?e, "Count query failed");
            ActorError::new(ActorErrorKind::KnowledgeTableNotFound(format!(
                "{}: {}",
                self.name, e
            )))
        })?;

        tracing::debug!(count = result.count, "Row count retrieved");
        Ok(result.count)
    }

    /// Check if table exists.
    ///
    /// # Arguments
    ///
    /// * `conn` - Database connection
    ///
    /// # Returns
    ///
    /// True if table exists, false otherwise.
    #[tracing::instrument(skip(self, conn), fields(table_name = %self.name))]
    pub fn exists(&self, conn: &mut PgConnection) -> bool {
        let query = format!(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '{}')",
            self.name
        );

        tracing::debug!(sql = %query, "Checking table existence");

        let result: Result<ExistsRow, _> = diesel::sql_query(&query).get_result(conn);

        match result {
            Ok(row) => {
                tracing::debug!(exists = row.exists, "Table existence checked");
                row.exists
            }
            Err(e) => {
                tracing::warn!(error = ?e, "Failed to check table existence");
                false
            }
        }
    }
}

/// Row result for JSON queries.
#[derive(Debug, QueryableByName)]
struct QueryRow {
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    data: JsonValue,
}

/// Row result for count queries.
#[derive(Debug, QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

/// Row result for existence checks.
#[derive(Debug, QueryableByName)]
struct ExistsRow {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    exists: bool,
}
