//! Schema inference from JSON structures.
//!
//! This module provides automatic schema inference from LLM-generated JSON,
//! allowing content generation without explicit template definitions.

use crate::{DatabaseError, DatabaseErrorKind, DatabaseResult};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Inferred column definition from JSON analysis
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDefinition {
    /// PostgreSQL data type
    pub pg_type: String,
    /// Whether the column is nullable
    pub nullable: bool,
    /// Example values seen (for debugging and type refinement)
    pub examples: Vec<JsonValue>,
}

impl ColumnDefinition {
    /// Create a new column definition
    pub fn new(pg_type: impl Into<String>, nullable: bool) -> Self {
        Self {
            pg_type: pg_type.into(),
            nullable,
            examples: Vec::new(),
        }
    }

    /// Add an example value
    pub fn add_example(&mut self, value: JsonValue) {
        self.examples.push(value);
    }
}

/// Inferred schema from JSON structure
#[derive(Debug, Clone)]
pub struct InferredSchema {
    /// Map of field names to column definitions
    pub fields: HashMap<String, ColumnDefinition>,
}

impl InferredSchema {
    /// Create a new empty schema
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Add a field from a JSON value
    pub fn add_field(&mut self, name: &str, value: &JsonValue) -> DatabaseResult<()> {
        let (pg_type, is_null) = infer_column_type(value);

        if let Some(existing) = self.fields.get_mut(name) {
            // Field seen before - refine type
            if is_null {
                tracing::trace!(field = name, "Marking field as nullable");
                existing.nullable = true;
            }
            existing.add_example(value.clone());

            // Type conflict resolution (e.g., BIGINT vs DOUBLE PRECISION)
            if existing.pg_type != pg_type {
                let resolved = resolve_type_conflict(&existing.pg_type, pg_type)?;

                if resolved != existing.pg_type {
                    tracing::warn!(
                        field = name,
                        from_type = existing.pg_type,
                        to_type = resolved,
                        "Type conflict resolved by widening"
                    );
                }

                existing.pg_type = resolved;
            }
        } else {
            // New field
            tracing::trace!(
                field = name,
                pg_type = pg_type,
                nullable = is_null,
                "Adding new field"
            );
            let mut def = ColumnDefinition::new(pg_type, is_null);
            def.add_example(value.clone());
            self.fields.insert(name.to_string(), def);
        }

        Ok(())
    }

    /// Get the number of fields in the schema
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Check if a field exists
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }
}

impl Default for InferredSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// Infer PostgreSQL column type from JSON value
///
/// Returns (type_name, is_null) tuple
pub fn infer_column_type(value: &JsonValue) -> (&'static str, bool) {
    match value {
        JsonValue::String(_) => ("TEXT", false),
        JsonValue::Number(n) => {
            if n.is_i64() || n.is_u64() {
                ("BIGINT", false)
            } else {
                ("DOUBLE PRECISION", false)
            }
        }
        JsonValue::Bool(_) => ("BOOLEAN", false),
        JsonValue::Null => ("TEXT", true), // Nullable, type inferred from other rows
        JsonValue::Array(arr) => {
            if arr.is_empty() {
                ("JSONB", true) // Unknown array type
            } else {
                // Check first element to determine array type
                match &arr[0] {
                    JsonValue::String(_) => ("TEXT[]", false),
                    JsonValue::Number(n) => {
                        if n.is_i64() || n.is_u64() {
                            ("BIGINT[]", false)
                        } else {
                            ("DOUBLE PRECISION[]", false)
                        }
                    }
                    JsonValue::Bool(_) => ("BOOLEAN[]", false),
                    _ => ("JSONB", false), // Complex array
                }
            }
        }
        JsonValue::Object(_) => ("JSONB", false),
    }
}

/// Resolve conflicts when same field has different types across rows
pub fn resolve_type_conflict(type1: &str, type2: &str) -> DatabaseResult<String> {
    // Same type - no conflict
    if type1 == type2 {
        return Ok(type1.to_string());
    }

    match (type1, type2) {
        // BIGINT vs DOUBLE PRECISION → DOUBLE PRECISION (wider type)
        ("BIGINT", "DOUBLE PRECISION") | ("DOUBLE PRECISION", "BIGINT") => {
            Ok("DOUBLE PRECISION".to_string())
        }
        // TEXT vs anything → TEXT (universal fallback)
        ("TEXT", _) | (_, "TEXT") => Ok("TEXT".to_string()),
        // JSONB vs anything → JSONB (universal structured fallback)
        ("JSONB", _) | (_, "JSONB") => Ok("JSONB".to_string()),
        // Array types must match exactly
        (a, b) if a.ends_with("[]") && b.ends_with("[]") => {
            if a == b {
                Ok(a.to_string())
            } else {
                Ok("JSONB".to_string()) // Heterogeneous array → JSONB
            }
        }
        // BIGINT compatible with integer types
        ("BIGINT", "INTEGER") | ("INTEGER", "BIGINT") => Ok("BIGINT".to_string()),
        // DOUBLE PRECISION compatible with numeric types
        ("DOUBLE PRECISION", "INTEGER") | ("INTEGER", "DOUBLE PRECISION") => {
            Ok("DOUBLE PRECISION".to_string())
        }
        // Boolean conflicts require fallback
        ("BOOLEAN", _) | (_, "BOOLEAN") => Ok("TEXT".to_string()),
        // All other incompatible types → TEXT fallback
        _ => Ok("TEXT".to_string()),
    }
}

/// Infer schema from JSON (single object or array)
pub fn infer_schema(json: &JsonValue) -> DatabaseResult<InferredSchema> {
    let items: Vec<&JsonValue> = match json {
        JsonValue::Object(_) => {
            tracing::debug!("Inferring schema from single JSON object");
            vec![json]
        }
        JsonValue::Array(arr) => {
            if arr.is_empty() {
                tracing::error!("Cannot infer schema from empty JSON array");
                return Err(DatabaseError::new(DatabaseErrorKind::SchemaInference(
                    "Cannot infer schema from empty JSON array. Hint: Ensure the LLM returns at least one object.".to_string(),
                )));
            }
            tracing::debug!(count = arr.len(), "Inferring schema from JSON array");
            arr.iter().collect()
        }
        _ => {
            tracing::error!(json_type = ?json, "Invalid JSON type for schema inference");
            return Err(DatabaseError::new(DatabaseErrorKind::SchemaInference(
                "Schema inference requires JSON object or array. Hint: Ensure the LLM returns structured JSON, not primitives.".to_string(),
            )));
        }
    };

    let mut schema = InferredSchema::new();

    for (idx, item) in items.iter().enumerate() {
        let obj = item.as_object().ok_or_else(|| {
            tracing::error!(index = idx, "Array item is not an object");
            DatabaseError::new(DatabaseErrorKind::SchemaInference(
                format!("Array item {} is not an object. Hint: Ensure all array elements are JSON objects with the same structure.", idx),
            ))
        })?;

        tracing::trace!(
            index = idx,
            field_count = obj.len(),
            "Processing object fields"
        );

        for (key, value) in obj {
            schema.add_field(key, value)?;
        }
    }

    tracing::info!(
        field_count = schema.field_count(),
        "Schema inference complete"
    );

    Ok(schema)
}

/// Create a table with inferred schema from JSON structure
///
/// This function creates a PostgreSQL table based on an inferred schema,
/// adding standard metadata columns for content generation tracking.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `table_name` - Name of the table to create
/// * `schema` - Inferred schema with column definitions
/// * `narrative_name` - Optional narrative file name for tracking
/// * `description` - Optional table description
///
/// # Returns
///
/// Returns `Ok(())` if the table was created successfully, or an error if creation failed.
#[cfg(feature = "database")]
pub fn create_inferred_table(
    conn: &mut diesel::pg::PgConnection,
    table_name: &str,
    schema: &InferredSchema,
    narrative_name: Option<&str>,
    description: Option<&str>,
) -> DatabaseResult<()> {
    use diesel::prelude::*;

    // Build column definitions
    let mut columns = Vec::new();

    for (name, def) in &schema.fields {
        let nullable = if def.nullable { "NULL" } else { "NOT NULL" };
        columns.push(format!("{} {} {}", name, def.pg_type, nullable));
    }

    // Add metadata columns (same as template-based tables)
    columns.push("generated_at TIMESTAMP NOT NULL DEFAULT NOW()".to_string());
    columns.push("source_narrative TEXT".to_string());
    columns.push("source_act TEXT".to_string());
    columns.push("generation_model TEXT".to_string());
    columns.push("review_status TEXT DEFAULT 'pending'".to_string());
    columns.push("tags TEXT[]".to_string());
    columns.push("rating INTEGER".to_string());

    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS {} ({})",
        table_name,
        columns.join(", ")
    );

    tracing::debug!(sql = %create_sql, "Creating inferred table");

    diesel::sql_query(&create_sql).execute(conn)?;

    tracing::info!(
        table = table_name,
        columns = schema.field_count(),
        "Inferred table created"
    );

    // Track in metadata table
    let narrative_value = narrative_name
        .map(|n| format!("'{}'", n.replace('\'', "''")))
        .unwrap_or_else(|| "NULL".to_string());

    let description_value = description
        .map(|d| format!("'{}'", d.replace('\'', "''")))
        .unwrap_or_else(|| "NULL".to_string());

    let insert_metadata = format!(
        "INSERT INTO content_generation_tables (table_name, template_source, narrative_file, description)
         VALUES ('{}', 'inferred', {}, {})
         ON CONFLICT (table_name) DO NOTHING",
        table_name.replace('\'', "''"),
        narrative_value,
        description_value,
    );

    diesel::sql_query(&insert_metadata).execute(conn)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_infer_column_type_string() {
        let (pg_type, nullable) = infer_column_type(&json!("hello"));
        assert_eq!(pg_type, "TEXT");
        assert!(!nullable);
    }

    #[test]
    fn test_infer_column_type_integer() {
        let (pg_type, nullable) = infer_column_type(&json!(42));
        assert_eq!(pg_type, "BIGINT");
        assert!(!nullable);
    }

    #[test]
    fn test_infer_column_type_float() {
        let (pg_type, nullable) = infer_column_type(&json!(3.15)); // Not PI
        assert_eq!(pg_type, "DOUBLE PRECISION");
        assert!(!nullable);
    }

    #[test]
    fn test_infer_column_type_boolean() {
        let (pg_type, nullable) = infer_column_type(&json!(true));
        assert_eq!(pg_type, "BOOLEAN");
        assert!(!nullable);
    }

    #[test]
    fn test_infer_column_type_null() {
        let (pg_type, nullable) = infer_column_type(&json!(null));
        assert_eq!(pg_type, "TEXT");
        assert!(nullable);
    }

    #[test]
    fn test_infer_column_type_string_array() {
        let (pg_type, nullable) = infer_column_type(&json!(["a", "b", "c"]));
        assert_eq!(pg_type, "TEXT[]");
        assert!(!nullable);
    }

    #[test]
    fn test_infer_column_type_number_array() {
        let (pg_type, nullable) = infer_column_type(&json!([1, 2, 3]));
        assert_eq!(pg_type, "BIGINT[]");
        assert!(!nullable);
    }

    #[test]
    fn test_infer_column_type_boolean_array() {
        let (pg_type, nullable) = infer_column_type(&json!([true, false]));
        assert_eq!(pg_type, "BOOLEAN[]");
        assert!(!nullable);
    }

    #[test]
    fn test_infer_column_type_empty_array() {
        let (pg_type, nullable) = infer_column_type(&json!([]));
        assert_eq!(pg_type, "JSONB");
        assert!(nullable);
    }

    #[test]
    fn test_infer_column_type_object() {
        let (pg_type, nullable) = infer_column_type(&json!({"key": "value"}));
        assert_eq!(pg_type, "JSONB");
        assert!(!nullable);
    }

    #[test]
    fn test_infer_column_type_complex_array() {
        let (pg_type, nullable) = infer_column_type(&json!([{"a": 1}, {"b": 2}]));
        assert_eq!(pg_type, "JSONB");
        assert!(!nullable);
    }

    #[test]
    fn test_resolve_type_conflict_same() {
        let result = resolve_type_conflict("TEXT", "TEXT").unwrap();
        assert_eq!(result, "TEXT");
    }

    #[test]
    fn test_resolve_type_conflict_bigint_double() {
        let result = resolve_type_conflict("BIGINT", "DOUBLE PRECISION").unwrap();
        assert_eq!(result, "DOUBLE PRECISION");
    }

    #[test]
    fn test_resolve_type_conflict_double_bigint() {
        let result = resolve_type_conflict("DOUBLE PRECISION", "BIGINT").unwrap();
        assert_eq!(result, "DOUBLE PRECISION");
    }

    #[test]
    fn test_resolve_type_conflict_text_fallback() {
        let result = resolve_type_conflict("BIGINT", "TEXT").unwrap();
        assert_eq!(result, "TEXT");
    }

    #[test]
    fn test_resolve_type_conflict_array_mismatch() {
        let result = resolve_type_conflict("TEXT[]", "BIGINT[]").unwrap();
        assert_eq!(result, "JSONB");
    }

    #[test]
    fn test_resolve_type_conflict_array_match() {
        let result = resolve_type_conflict("TEXT[]", "TEXT[]").unwrap();
        assert_eq!(result, "TEXT[]");
    }

    #[test]
    fn test_resolve_type_conflict_boolean() {
        let result = resolve_type_conflict("BOOLEAN", "BIGINT").unwrap();
        assert_eq!(result, "TEXT");
    }

    #[test]
    fn test_infer_schema_simple_object() {
        let json = json!({
            "name": "Alice",
            "age": 30,
            "active": true
        });
        let schema = infer_schema(&json).unwrap();
        assert_eq!(schema.field_count(), 3);
        assert_eq!(schema.fields["name"].pg_type, "TEXT");
        assert_eq!(schema.fields["age"].pg_type, "BIGINT");
        assert_eq!(schema.fields["active"].pg_type, "BOOLEAN");
    }

    #[test]
    fn test_infer_schema_with_nulls() {
        let json = json!([
            { "name": "Alice", "email": null },
            { "name": "Bob", "email": "bob@example.com" }
        ]);
        let schema = infer_schema(&json).unwrap();
        assert_eq!(schema.field_count(), 2);
        assert!(schema.fields["email"].nullable);
        assert_eq!(schema.fields["email"].pg_type, "TEXT");
    }

    #[test]
    fn test_infer_schema_type_conflict_bigint_to_double() {
        let json = json!([
            { "value": 42 },
            { "value": 3.15 } // Not PI
        ]);
        let schema = infer_schema(&json).unwrap();
        assert_eq!(schema.field_count(), 1);
        assert_eq!(schema.fields["value"].pg_type, "DOUBLE PRECISION");
    }

    #[test]
    fn test_infer_schema_array() {
        let json = json!([
            { "id": 1, "name": "Alice" },
            { "id": 2, "name": "Bob" },
            { "id": 3, "name": "Charlie" }
        ]);
        let schema = infer_schema(&json).unwrap();
        assert_eq!(schema.field_count(), 2);
        assert_eq!(schema.fields["id"].pg_type, "BIGINT");
        assert_eq!(schema.fields["name"].pg_type, "TEXT");
        assert_eq!(schema.fields["id"].examples.len(), 3);
    }

    #[test]
    fn test_infer_schema_empty_array_error() {
        let json = json!([]);
        let result = infer_schema(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_infer_schema_non_object_error() {
        let json = json!("not an object");
        let result = infer_schema(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_infer_schema_array_with_non_object_error() {
        let json = json!([1, 2, 3]);
        let result = infer_schema(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_column_definition_creation() {
        let def = ColumnDefinition::new("TEXT", false);
        assert_eq!(def.pg_type, "TEXT");
        assert!(!def.nullable);
        assert_eq!(def.examples.len(), 0);
    }

    #[test]
    fn test_column_definition_add_example() {
        let mut def = ColumnDefinition::new("TEXT", false);
        def.add_example(json!("test"));
        assert_eq!(def.examples.len(), 1);
        assert_eq!(def.examples[0], json!("test"));
    }

    #[test]
    fn test_inferred_schema_has_field() {
        let json = json!({"name": "Alice", "age": 30});
        let schema = infer_schema(&json).unwrap();
        assert!(schema.has_field("name"));
        assert!(schema.has_field("age"));
        assert!(!schema.has_field("email"));
    }

    #[test]
    fn test_inferred_schema_add_field_directly() {
        let mut schema = InferredSchema::new();
        schema.add_field("test", &json!("value")).unwrap();
        assert_eq!(schema.field_count(), 1);
        assert_eq!(schema.fields["test"].pg_type, "TEXT");
    }

    #[test]
    fn test_infer_schema_with_nested_objects() {
        let json = json!({
            "id": 1,
            "metadata": {"created": "2025-01-01", "author": "Alice"}
        });
        let schema = infer_schema(&json).unwrap();
        assert_eq!(schema.field_count(), 2);
        assert_eq!(schema.fields["id"].pg_type, "BIGINT");
        assert_eq!(schema.fields["metadata"].pg_type, "JSONB");
    }

    #[test]
    fn test_infer_schema_with_mixed_array() {
        let json = json!({
            "id": 1,
            "tags": ["rust", "database", "llm"]
        });
        let schema = infer_schema(&json).unwrap();
        assert_eq!(schema.field_count(), 2);
        assert_eq!(schema.fields["tags"].pg_type, "TEXT[]");
    }

    #[test]
    fn test_resolve_type_conflict_integer_types() {
        let result = resolve_type_conflict("INTEGER", "BIGINT").unwrap();
        assert_eq!(result, "BIGINT");
    }

    #[test]
    fn test_resolve_type_conflict_integer_double() {
        let result = resolve_type_conflict("INTEGER", "DOUBLE PRECISION").unwrap();
        assert_eq!(result, "DOUBLE PRECISION");
    }

    #[test]
    fn test_infer_schema_multiple_objects_consolidation() {
        let json = json!([
            { "a": 1, "b": "x" },
            { "a": 2, "c": true },
            { "a": 3, "b": "y", "c": false }
        ]);
        let schema = infer_schema(&json).unwrap();
        // Schema consolidation discovers all fields across all objects
        assert_eq!(schema.field_count(), 3);
        assert!(schema.has_field("a"));
        assert!(schema.has_field("b"));
        assert!(schema.has_field("c"));
        // Types should be inferred correctly
        assert_eq!(schema.fields["a"].pg_type, "BIGINT");
        assert_eq!(schema.fields["b"].pg_type, "TEXT");
        assert_eq!(schema.fields["c"].pg_type, "BOOLEAN");
    }

    #[test]
    fn test_column_definition_multiple_examples() {
        let mut def = ColumnDefinition::new("TEXT", false);
        def.add_example(json!("test1"));
        def.add_example(json!("test2"));
        def.add_example(json!("test3"));
        assert_eq!(def.examples.len(), 3);
    }

    #[test]
    fn test_infer_column_type_float_array() {
        let (pg_type, nullable) = infer_column_type(&json!([1.1, 2.2, 3.3]));
        assert_eq!(pg_type, "DOUBLE PRECISION[]");
        assert!(!nullable);
    }
}
