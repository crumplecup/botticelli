//! Table query execution for narrative table references.

use crate::{DatabaseError, DatabaseErrorKind, DatabaseResult};
use botticelli_interface::{TableCountView, TableQueryView};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use tracing::{debug, instrument};

/// Executes table queries for narrative table references.
#[derive(Clone, derive_getters::Getters)]
pub struct TableQueryExecutor {
    connection: Arc<Mutex<PgConnection>>,
}

impl TableQueryExecutor {
    /// Creates a new table query executor.
    pub fn new(connection: Arc<Mutex<PgConnection>>) -> Self {
        Self { connection }
    }

    /// Queries a table and returns results as JSON values.
    #[instrument(skip(self), fields(table_name = %view.table_name(), limit = ?view.limit(), offset = ?view.offset()))]
    pub fn query_table(&self, view: &TableQueryView) -> DatabaseResult<Vec<JsonValue>> {
        debug!("Querying table");

        let mut conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Connection(e.to_string())))?;

        // Validate table exists
        if !self.table_exists(&mut conn, view.table_name())? {
            return Err(DatabaseError::new(DatabaseErrorKind::TableNotFound(
                view.table_name().to_string(),
            )));
        }

        // Build SQL query
        let query = self.build_query(view)?;

        debug!(query = %query, "Executing table query");

        // Execute query using raw SQL
        let results = self.execute_raw_query(&mut conn, &query)?;

        debug!(count = results.len(), "Retrieved rows");
        Ok(results)
    }

    /// Queries a table, returns results, and deletes those rows (destructive read).
    #[instrument(skip(self), fields(table_name = %view.table_name(), limit = ?view.limit(), offset = ?view.offset()))]
    pub fn query_and_delete_table(&self, view: &TableQueryView) -> DatabaseResult<Vec<JsonValue>> {
        debug!("Querying and deleting from table");

        let mut conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Connection(e.to_string())))?;

        // Validate table exists
        if !self.table_exists(&mut conn, view.table_name())? {
            return Err(DatabaseError::new(DatabaseErrorKind::TableNotFound(
                view.table_name().to_string(),
            )));
        }

        // Call pull_and_delete from content_management
        let limit = view.limit().unwrap_or(10) as usize;
        let results =
            crate::content_management::pull_and_delete(&mut conn, view.table_name(), limit)
                .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))?;

        debug!(count = results.len(), "Retrieved and deleted rows");
        Ok(results)
    }

    /// Checks if a table exists in the database.
    #[instrument(skip(self, conn))]
    fn table_exists(&self, conn: &mut PgConnection, table_name: &str) -> DatabaseResult<bool> {
        #[derive(QueryableByName)]
        struct ExistsResult {
            #[diesel(sql_type = diesel::sql_types::Bool)]
            exists: bool,
        }

        let query = "SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_name = $1
        ) as exists";

        let result: ExistsResult = diesel::sql_query(query)
            .bind::<Text, _>(table_name)
            .get_result(conn)
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))?;

        Ok(result.exists)
    }

    /// Builds a SELECT query from the provided view.
    fn build_query(&self, view: &TableQueryView) -> DatabaseResult<String> {
        let table_name = view.table_name();

        // Sanitize table name (alphanumeric and underscores only)
        if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(DatabaseError::new(DatabaseErrorKind::InvalidQuery(
                "Table name contains invalid characters".into(),
            )));
        }

        let col_list = if let Some(cols) = view.columns() {
            // Sanitize column names
            for col in cols {
                if !col.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return Err(DatabaseError::new(DatabaseErrorKind::InvalidQuery(
                        format!("Column name '{}' contains invalid characters", col),
                    )));
                }
            }
            cols.join(", ")
        } else {
            "*".to_string()
        };

        let mut query = format!("SELECT {} FROM {}", col_list, table_name);

        if let Some(where_clause) = view.filter() {
            let safe_clause = self.sanitize_where_clause(where_clause)?;
            query.push_str(&format!(" WHERE {}", safe_clause));
        }

        if let Some(order) = view.order_by() {
            // Basic sanitization for ORDER BY
            if !order
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ' ' || c == ',')
            {
                return Err(DatabaseError::new(DatabaseErrorKind::InvalidQuery(
                    "ORDER BY contains invalid characters".into(),
                )));
            }
            query.push_str(&format!(" ORDER BY {}", order));
        }

        if let Some(lim) = view.limit() {
            query.push_str(&format!(" LIMIT {}", lim));
        }

        if let Some(off) = view.offset() {
            query.push_str(&format!(" OFFSET {}", off));
        }

        Ok(query)
    }

    /// Sanitizes a WHERE clause to prevent SQL injection.
    fn sanitize_where_clause(&self, clause: &str) -> DatabaseResult<String> {
        // Basic SQL injection prevention
        // Check for dangerous patterns
        if clause.contains(';') || clause.contains("--") || clause.to_lowercase().contains("drop ")
        {
            return Err(DatabaseError::new(DatabaseErrorKind::InvalidQuery(
                "WHERE clause contains unsafe patterns".into(),
            )));
        }

        // This is a basic check. In production, use parameterized queries
        // or a proper SQL parser
        Ok(clause.to_string())
    }

    /// Executes a raw SQL query and returns results as JSON.
    fn execute_raw_query(
        &self,
        conn: &mut PgConnection,
        query: &str,
    ) -> DatabaseResult<Vec<JsonValue>> {
        use tracing::warn;

        // Use diesel's sql_query to execute raw SQL
        // We'll return the results as JSON strings from PostgreSQL
        let json_query = format!("SELECT row_to_json(t) as json FROM ({}) t", query);

        #[derive(QueryableByName)]
        struct JsonRow {
            #[diesel(sql_type = diesel::sql_types::Json)]
            json: JsonValue,
        }

        match diesel::sql_query(&json_query).load::<JsonRow>(conn) {
            Ok(results) => Ok(results.into_iter().map(|row| row.json).collect()),
            Err(e) => {
                let err_msg = e.to_string();
                // Handle missing column errors gracefully
                if err_msg.contains("column") && err_msg.contains("does not exist") {
                    warn!(
                        error = %err_msg,
                        query = %query,
                        "Query references non-existent column - returning empty result set"
                    );
                    // Return empty result instead of propagating error
                    Ok(Vec::new())
                } else {
                    Err(DatabaseError::new(DatabaseErrorKind::Query(err_msg)))
                }
            }
        }
    }

    /// Gets the count of rows that would be returned by a query.
    #[instrument(skip(self), fields(table_name = %view.table_name()))]
    pub fn count_rows(&self, view: &TableCountView) -> DatabaseResult<i64> {
        let mut conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Connection(e.to_string())))?;

        let table_name = view.table_name();

        // Validate table exists
        if !self.table_exists(&mut conn, table_name)? {
            return Err(DatabaseError::new(DatabaseErrorKind::TableNotFound(
                table_name.to_string(),
            )));
        }

        let mut query = format!("SELECT COUNT(*) as count FROM {}", table_name);

        if let Some(where_clause) = view.filter() {
            let safe_clause = self.sanitize_where_clause(where_clause)?;
            query.push_str(&format!(" WHERE {}", safe_clause));
        }

        debug!(query = %query, "Counting rows");

        #[derive(QueryableByName)]
        struct CountResult {
            #[diesel(sql_type = BigInt)]
            count: i64,
        }

        let result: CountResult = diesel::sql_query(&query)
            .get_result(&mut *conn)
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))?;

        Ok(result.count)
    }
}

/// Formats table results as JSON.
pub fn format_as_json(rows: &[JsonValue]) -> String {
    serde_json::to_string_pretty(rows).unwrap_or_else(|_| "[]".to_string())
}

/// Formats table results as Markdown table.
pub fn format_as_markdown(rows: &[JsonValue]) -> String {
    if rows.is_empty() {
        return "No data".to_string();
    }

    // Extract column names from first row
    let first = &rows[0];
    let columns: Vec<String> = if let Some(obj) = first.as_object() {
        obj.keys().cloned().collect()
    } else {
        return "Invalid data format".to_string();
    };

    if columns.is_empty() {
        return "No columns".to_string();
    }

    let mut output = String::new();

    // Header row
    output.push_str("| ");
    output.push_str(&columns.join(" | "));
    output.push_str(" |\n");

    // Separator
    output.push('|');
    for _ in &columns {
        output.push_str(" --- |");
    }
    output.push('\n');

    // Data rows
    for row in rows {
        if let Some(obj) = row.as_object() {
            output.push_str("| ");
            let values: Vec<String> = columns
                .iter()
                .map(|col| {
                    obj.get(col)
                        .map(|v| match v {
                            JsonValue::String(s) => s.clone(),
                            JsonValue::Number(n) => n.to_string(),
                            JsonValue::Bool(b) => b.to_string(),
                            JsonValue::Null => "null".to_string(),
                            _ => serde_json::to_string(v).unwrap_or_default(),
                        })
                        .unwrap_or_else(|| "".to_string())
                })
                .collect();
            output.push_str(&values.join(" | "));
            output.push_str(" |\n");
        }
    }

    output
}

/// Formats table results as CSV.
pub fn format_as_csv(rows: &[JsonValue]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    // Extract column names from first row
    let first = &rows[0];
    let columns: Vec<String> = if let Some(obj) = first.as_object() {
        obj.keys().cloned().collect()
    } else {
        return "Invalid data format\n".to_string();
    };

    if columns.is_empty() {
        return "No columns\n".to_string();
    }

    let mut output = String::new();

    // Header row
    output.push_str(&columns.join(","));
    output.push('\n');

    // Data rows
    for row in rows {
        if let Some(obj) = row.as_object() {
            let values: Vec<String> = columns
                .iter()
                .map(|col| {
                    obj.get(col)
                        .map(|v| match v {
                            JsonValue::String(s) => {
                                // Escape quotes and wrap in quotes if contains comma
                                if s.contains(',') || s.contains('"') || s.contains('\n') {
                                    format!("\"{}\"", s.replace('"', "\"\""))
                                } else {
                                    s.clone()
                                }
                            }
                            JsonValue::Number(n) => n.to_string(),
                            JsonValue::Bool(b) => b.to_string(),
                            JsonValue::Null => String::new(),
                            _ => serde_json::to_string(v).unwrap_or_default(),
                        })
                        .unwrap_or_default()
                })
                .collect();
            output.push_str(&values.join(","));
            output.push('\n');
        }
    }

    output
}
