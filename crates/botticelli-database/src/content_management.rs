//! Content management for generated content tables.
//!
//! Provides functions for querying, updating, and managing content
//! in dynamically created generation tables.

use crate::schema_reflection::reflect_table_schema;
use crate::{BotticelliResult, DatabaseError, DatabaseErrorKind};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

/// List generated content from a table.
///
/// Queries a dynamically named content table and returns results as JSON.
/// Supports filtering by review_status and limiting results.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `table_name` - Name of the content table
/// * `status_filter` - Optional review status filter ("pending", "approved", "rejected")
/// * `limit` - Maximum number of results to return
///
/// # Returns
///
/// Vector of JSON objects representing table rows
pub fn list_content(
    conn: &mut PgConnection,
    table_name: &str,
    status_filter: Option<&str>,
    limit: usize,
) -> BotticelliResult<Vec<JsonValue>> {
    // Build query dynamically
    let mut query = format!(
        "SELECT row_to_json(t) FROM (SELECT * FROM {} WHERE 1=1",
        table_name
    );

    if let Some(status) = status_filter {
        query.push_str(&format!(" AND review_status = '{}'", status));
    }

    query.push_str(" ORDER BY generated_at DESC");
    query.push_str(&format!(" LIMIT {}", limit));
    query.push_str(") t");

    tracing::debug!(sql = %query, "Listing content");

    // Execute query and collect results
    let results: Vec<String> = diesel::sql_query(&query)
        .load::<StringRow>(conn)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))?
        .into_iter()
        .map(|row| row.row_to_json)
        .collect();

    // Parse JSON strings
    let json_results: Result<Vec<JsonValue>, _> =
        results.iter().map(|s| serde_json::from_str(s)).collect();

    json_results.map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())).into())
}

/// Get a specific content item by ID.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `table_name` - Name of the content table
/// * `id` - Content ID
///
/// # Returns
///
/// JSON object representing the row, or error if not found
pub fn get_content_by_id(
    conn: &mut PgConnection,
    table_name: &str,
    id: i64,
) -> BotticelliResult<JsonValue> {
    let query = format!(
        "SELECT row_to_json(t) FROM (SELECT * FROM {} WHERE id = {}) t",
        table_name, id
    );

    tracing::debug!(sql = %query, "Getting content by ID");

    let result: String = diesel::sql_query(&query)
        .get_result::<StringRow>(conn)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))?
        .row_to_json;

    serde_json::from_str(&result)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())).into())
}

/// Update tags and rating for a content item.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `table_name` - Name of the content table
/// * `id` - Content ID
/// * `tags` - Optional tags to set (replaces existing)
/// * `rating` - Optional rating (1-5)
pub fn update_content_metadata(
    conn: &mut PgConnection,
    table_name: &str,
    id: i64,
    tags: Option<&[String]>,
    rating: Option<i32>,
) -> BotticelliResult<()> {
    let mut updates = Vec::new();

    if let Some(tag_list) = tags {
        let tags_sql = if tag_list.is_empty() {
            "NULL".to_string()
        } else {
            let escaped: Vec<String> = tag_list
                .iter()
                .map(|t| format!("'{}'", t.replace('\'', "''")))
                .collect();
            format!("ARRAY[{}]", escaped.join(", "))
        };
        updates.push(format!("tags = {}", tags_sql));
    }

    if let Some(r) = rating {
        if !(1..=5).contains(&r) {
            return Err(DatabaseError::new(DatabaseErrorKind::Query(
                "Rating must be between 1 and 5".to_string(),
            ))
            .into());
        }
        updates.push(format!("rating = {}", r));
    }

    if updates.is_empty() {
        return Ok(());
    }

    let query = format!(
        "UPDATE {} SET {} WHERE id = {}",
        table_name,
        updates.join(", "),
        id
    );

    tracing::debug!(sql = %query, "Updating content metadata");

    diesel::sql_query(&query)
        .execute(conn)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))?;

    Ok(())
}

/// Update review status for a content item.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `table_name` - Name of the content table
/// * `id` - Content ID
/// * `status` - New status ("pending", "approved", "rejected")
pub fn update_review_status(
    conn: &mut PgConnection,
    table_name: &str,
    id: i64,
    status: &str,
) -> BotticelliResult<()> {
    // Validate status
    if !["pending", "approved", "rejected"].contains(&status) {
        return Err(DatabaseError::new(DatabaseErrorKind::Query(
            "Status must be 'pending', 'approved', or 'rejected'".to_string(),
        ))
        .into());
    }

    let query = format!(
        "UPDATE {} SET review_status = '{}' WHERE id = {}",
        table_name, status, id
    );

    tracing::debug!(sql = %query, "Updating review status");

    diesel::sql_query(&query)
        .execute(conn)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))?;

    Ok(())
}

/// Delete a content item.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `table_name` - Name of the content table
/// * `id` - Content ID
pub fn delete_content(conn: &mut PgConnection, table_name: &str, id: i64) -> BotticelliResult<()> {
    let query = format!("DELETE FROM {} WHERE id = {}", table_name, id);

    tracing::debug!(sql = %query, "Deleting content");

    diesel::sql_query(&query)
        .execute(conn)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))?;

    Ok(())
}

/// Promote content to a production table.
///
/// Copies a content item from a generation table to a production table,
/// stripping metadata columns and handling foreign key relationships.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `source_table` - Source generation table name
/// * `target_table` - Target production table name (defaults to template table)
/// * `id` - Content ID to promote
///
/// # Returns
///
/// The ID of the inserted row in the target table
pub fn promote_content(
    conn: &mut PgConnection,
    source_table: &str,
    target_table: &str,
    id: i64,
) -> BotticelliResult<i64> {
    tracing::info!(
        source = source_table,
        target = target_table,
        id = id,
        "Promoting content"
    );

    // Get the source content
    let content = get_content_by_id(conn, source_table, id)?;

    // Get target table schema to determine which columns to copy
    let target_schema = reflect_table_schema(conn, target_table)?;

    // Build column list (exclude metadata columns)
    let metadata_columns = [
        "generated_at",
        "source_narrative",
        "source_act",
        "generation_model",
        "review_status",
        "tags",
        "rating",
    ];

    let target_columns: Vec<String> = target_schema
        .columns
        .iter()
        .filter(|col| col.name != "id" && !metadata_columns.contains(&col.name.as_str()))
        .map(|col| col.name.clone())
        .collect();

    if target_columns.is_empty() {
        return Err(DatabaseError::new(DatabaseErrorKind::Query(
            "No columns to copy (target table has no matching columns)".to_string(),
        ))
        .into());
    }

    // Build values list
    let mut values = Vec::new();
    for col_name in &target_columns {
        if let Some(value) = content.get(col_name) {
            values.push(json_value_to_sql(value));
        } else {
            // Column exists in target but not in source - use NULL
            values.push("NULL".to_string());
        }
    }

    // Build INSERT query
    let insert_sql = format!(
        "INSERT INTO {} ({}) VALUES ({}) RETURNING id",
        target_table,
        target_columns.join(", "),
        values.join(", ")
    );

    tracing::debug!(sql = %insert_sql, "Inserting promoted content");

    // Execute and get the new ID
    let new_id: i64 = diesel::sql_query(&insert_sql)
        .get_result::<IdRow>(conn)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))?
        .id;

    tracing::info!(new_id = new_id, "Content promoted successfully");

    Ok(new_id)
}

/// Helper to convert JSON value to SQL string.
fn json_value_to_sql(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "NULL".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => format!("'{}'", s.replace('\'', "''")),
        JsonValue::Array(_) | JsonValue::Object(_) => {
            format!("'{}'::jsonb", value.to_string().replace('\'', "''"))
        }
    }
}

/// Helper struct for deserializing row_to_json results.
#[derive(QueryableByName)]
struct StringRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    row_to_json: String,
}

/// Helper struct for deserializing RETURNING id.
#[derive(QueryableByName)]
struct IdRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    id: i64,
}
