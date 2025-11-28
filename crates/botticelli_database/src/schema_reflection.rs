//! Schema reflection and dynamic table management for content generation.
//!
//! This module provides functionality to:
//! - Query PostgreSQL information_schema to inspect table structures
//! - Create new tables based on existing Discord table templates
//! - Add metadata columns for content generation tracking
//!
//! These functions are used by the content generation processor in Phase 2.

#![allow(dead_code)] // Phase 1: Infrastructure only, will be used in Phase 2

use crate::DatabaseResult;
use botticelli_error::{DatabaseError, DatabaseErrorKind};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use tracing::instrument;

/// Represents a database column's structure
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, QueryableByName)]
pub struct ColumnInfo {
    /// Column name
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
    /// PostgreSQL data type
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub data_type: String,
    /// Whether the column is nullable
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub is_nullable: String,
    /// Character maximum length (for varchar types)
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    pub character_maximum_length: Option<i32>,
    /// Default value expression
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub column_default: Option<String>,
}

/// Represents a table's schema structure
#[derive(Debug, Clone)]
pub struct TableSchema {
    /// Table name
    pub table_name: String,
    /// Columns in the table
    pub columns: Vec<ColumnInfo>,
}

/// Query information_schema to get column information for a table
#[instrument(name = "schema_reflection.reflect_table_schema", skip(conn), fields(table = %table_name))]
pub fn reflect_table_schema(
    conn: &mut PgConnection,
    table_name: &str,
) -> DatabaseResult<TableSchema> {
    let query = format!(
        r#"
        SELECT 
            column_name as name,
            CASE 
                WHEN data_type = 'ARRAY' THEN udt_name
                ELSE data_type
            END as data_type,
            is_nullable,
            character_maximum_length,
            column_default
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = '{}'
        ORDER BY ordinal_position
        "#,
        table_name
    );

    let results: Vec<ColumnInfo> = diesel::sql_query(&query).load(conn).map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Query(format!(
            "Failed to query schema for table '{}': {}",
            table_name, e
        )))
    })?;

    if results.is_empty() {
        return Err(DatabaseError::new(DatabaseErrorKind::TableNotFound(
            table_name.to_string(),
        )));
    }

    Ok(TableSchema {
        table_name: table_name.to_string(),
        columns: results,
    })
}

/// Generate CREATE TABLE SQL from a table schema
pub fn generate_create_table_sql(target_table_name: &str, source_schema: &TableSchema) -> String {
    let mut sql = format!("CREATE TABLE {} (\n", target_table_name);

    let column_defs: Vec<String> = source_schema
        .columns
        .iter()
        .map(|col| {
            let mut def = format!("    {}", col.name);

            // Handle varchar with length specially
            if col.data_type == "character varying" {
                if let Some(max_len) = col.character_maximum_length {
                    def.push_str(&format!(" VARCHAR({})", max_len));
                } else {
                    def.push_str(" VARCHAR");
                }
            } else {
                def.push_str(&format!(" {}", map_data_type(&col.data_type)));
            }

            // Make foreign keys nullable in content generation tables
            if col.name.ends_with("_id") && col.name != "id" {
                def.push_str(" NULL");
            } else if col.is_nullable != "YES" {
                def.push_str(" NOT NULL");
            }

            // Skip defaults that reference sequences (for serial columns)
            if let Some(default) = &col.column_default
                && !default.contains("nextval")
            {
                def.push_str(&format!(" DEFAULT {}", default));
            }

            def
        })
        .collect();

    sql.push_str(&column_defs.join(",\n"));

    // Add content generation metadata columns
    sql.push_str(",\n\n    -- Content generation metadata\n");
    sql.push_str("    generated_at TIMESTAMP NOT NULL DEFAULT NOW(),\n");
    sql.push_str("    source_narrative TEXT,\n");
    sql.push_str("    source_act TEXT,\n");
    sql.push_str("    generation_model TEXT,\n");
    sql.push_str("    review_status TEXT DEFAULT 'pending',\n");
    sql.push_str("    tags TEXT[],\n");
    sql.push_str("    rating INTEGER");

    sql.push_str("\n)");
    sql
}

/// Map PostgreSQL data types to Diesel-compatible types
fn map_data_type(pg_type: &str) -> &'static str {
    match pg_type {
        "bigint" => "BIGINT",
        "integer" => "INTEGER",
        "smallint" => "SMALLINT",
        "boolean" => "BOOLEAN",
        "text" => "TEXT",
        "character varying" => "VARCHAR",
        "timestamp without time zone" => "TIMESTAMP",
        "timestamp with time zone" => "TIMESTAMPTZ",
        "jsonb" => "JSONB",
        "uuid" => "UUID",
        "real" => "REAL",
        "double precision" => "DOUBLE PRECISION",
        "ARRAY" => "TEXT[]",
        _ => "TEXT", // Safe default
    }
}

/// Result struct for table existence check
#[derive(Debug, QueryableByName)]
struct TableExistsResult {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    exists: bool,
}

/// Check if a table exists in the database
#[instrument(name = "schema_reflection.table_exists", skip(conn), fields(table = %table_name))]
pub fn table_exists(conn: &mut PgConnection, table_name: &str) -> DatabaseResult<bool> {
    let query = format!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'public'
              AND table_name = '{}'
        ) as exists
        "#,
        table_name
    );

    let result: TableExistsResult = diesel::sql_query(&query).get_result(conn).map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Query(format!(
            "Failed to check table existence: {}",
            e
        )))
    })?;

    Ok(result.exists)
}

/// Create a content generation table based on a template
#[instrument(name = "schema_reflection.create_content_table", skip(conn), fields(table = %table_name, template = %template_source))]
pub fn create_content_table(
    conn: &mut PgConnection,
    table_name: &str,
    template_source: &str,
    narrative_file: Option<&str>,
    description: Option<&str>,
) -> DatabaseResult<()> {
    // Check if table already exists
    if table_exists(conn, table_name)? {
        tracing::info!(
            "Content generation table '{}' already exists, skipping creation",
            table_name
        );
        return Ok(());
    }

    // Reflect source table schema
    let source_schema = reflect_table_schema(conn, template_source)?;

    // Generate CREATE TABLE SQL
    let create_sql = generate_create_table_sql(table_name, &source_schema);

    // Execute CREATE TABLE
    diesel::sql_query(&create_sql).execute(conn).map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Query(format!(
            "Failed to create table '{}': {}",
            table_name, e
        )))
    })?;

    // Insert metadata record
    let insert_sql = format!(
        r#"
        INSERT INTO content_generation_tables (table_name, template_source, narrative_file, description)
        VALUES ('{}', '{}', {}, {})
        "#,
        table_name,
        template_source,
        narrative_file
            .map(|f| format!("'{}'", f))
            .unwrap_or_else(|| "NULL".to_string()),
        description
            .map(|d| format!("'{}'", d))
            .unwrap_or_else(|| "NULL".to_string())
    );

    diesel::sql_query(&insert_sql).execute(conn).map_err(|e| {
        DatabaseError::new(DatabaseErrorKind::Query(format!(
            "Failed to insert metadata for table '{}': {}",
            table_name, e
        )))
    })?;

    tracing::info!(
        "Created content generation table '{}' from template '{}'",
        table_name,
        template_source
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_data_type() {
        assert_eq!(map_data_type("bigint"), "BIGINT");
        assert_eq!(map_data_type("text"), "TEXT");
        assert_eq!(map_data_type("boolean"), "BOOLEAN");
        assert_eq!(map_data_type("unknown_type"), "TEXT");
    }

    #[test]
    fn test_generate_create_table_sql() {
        let schema = TableSchema {
            table_name: "test_source".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: "NO".to_string(),
                    character_maximum_length: None,
                    column_default: Some("nextval('seq'::regclass)".to_string()),
                },
                ColumnInfo {
                    name: "name".to_string(),
                    data_type: "character varying".to_string(),
                    is_nullable: "NO".to_string(),
                    character_maximum_length: Some(100),
                    column_default: None,
                },
                ColumnInfo {
                    name: "guild_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: "NO".to_string(),
                    character_maximum_length: None,
                    column_default: None,
                },
            ],
        };

        let sql = generate_create_table_sql("test_target", &schema);

        assert!(sql.contains("CREATE TABLE test_target"));
        assert!(sql.contains("id BIGINT NOT NULL"));
        assert!(sql.contains("name VARCHAR(100) NOT NULL"));
        assert!(sql.contains("guild_id BIGINT NULL")); // FK made nullable
        assert!(sql.contains("generated_at TIMESTAMP NOT NULL DEFAULT NOW()"));
        assert!(sql.contains("source_narrative TEXT"));
        assert!(sql.contains("review_status TEXT DEFAULT 'pending'"));
    }
}
