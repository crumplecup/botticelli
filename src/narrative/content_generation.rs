//! Content generation processor for creating reviewable content.
//!
//! This processor detects narratives with a `template` field and generates
//! content into custom tables based on Discord schema templates.

use crate::{ActProcessor, BoticelliResult, ProcessorContext, extract_json, parse_json};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

#[cfg(feature = "database")]
use crate::{PgConnection, create_content_table};
#[cfg(feature = "database")]
use diesel::prelude::*;
#[cfg(feature = "database")]
use std::sync::{Arc, Mutex};

/// Processor for content generation into custom tables.
///
/// Detects narratives with a `template` field and:
/// 1. Creates a custom table if it doesn't exist (based on template schema)
/// 2. Extracts JSON from LLM responses
/// 3. Inserts generated content with metadata columns
#[cfg(feature = "database")]
pub struct ContentGenerationProcessor {
    /// Database connection wrapped in Arc<Mutex> for thread safety
    connection: Arc<Mutex<PgConnection>>,
}

#[cfg(feature = "database")]
impl ContentGenerationProcessor {
    /// Create a new content generation processor.
    ///
    /// # Arguments
    ///
    /// * `connection` - Database connection for table creation and inserts
    pub fn new(connection: Arc<Mutex<PgConnection>>) -> Self {
        Self { connection }
    }

    /// Insert generated content into the target table with metadata.
    fn insert_content(
        &self,
        table_name: &str,
        json_data: &JsonValue,
        narrative_name: &str,
        act_name: &str,
        model: Option<&str>,
    ) -> BoticelliResult<()> {
        let mut conn = self
            .connection
            .lock()
            .map_err(|e| crate::BackendError::new(format!("Failed to lock connection: {}", e)))?;

        // Query schema to get column types
        let schema = crate::reflect_table_schema(&mut conn, table_name)?;
        let column_types: std::collections::HashMap<_, _> = schema
            .columns
            .iter()
            .map(|col| (col.name.as_str(), col.data_type.as_str()))
            .collect();

        // Build INSERT statement dynamically
        // Extract fields from JSON and add metadata columns
        let obj = json_data
            .as_object()
            .ok_or_else(|| crate::BackendError::new("JSON must be an object"))?;

        let mut columns = Vec::new();
        let mut values = Vec::new();

        // Add content fields from JSON
        for (key, value) in obj {
            columns.push(key.clone());
            let col_type = column_types.get(key.as_str()).copied().unwrap_or("text");
            values.push(json_value_to_sql(value, col_type));
        }

        // Add metadata columns
        columns.push("source_narrative".to_string());
        values.push(format!("'{}'", narrative_name));

        columns.push("source_act".to_string());
        values.push(format!("'{}'", act_name));

        if let Some(m) = model {
            columns.push("generation_model".to_string());
            values.push(format!("'{}'", m));
        }

        let insert_sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            values.join(", ")
        );

        tracing::debug!(sql = %insert_sql, "Inserting generated content");

        diesel::sql_query(&insert_sql)
            .execute(&mut *conn)
            .map_err(|e| crate::BackendError::new(format!("Failed to insert content: {}", e)))?;

        Ok(())
    }
}

#[cfg(feature = "database")]
#[async_trait]
impl ActProcessor for ContentGenerationProcessor {
    async fn process(&self, context: &ProcessorContext<'_>) -> BoticelliResult<()> {
        let template = context
            .narrative_metadata
            .template
            .as_ref()
            .expect("should_process ensures template exists");

        let table_name = &context.narrative_metadata.name;

        tracing::info!(
            act = %context.execution.act_name,
            table = %table_name,
            template = %template,
            "Processing content generation"
        );

        // Create table if needed
        {
            let mut conn = self.connection.lock().map_err(|e| {
                crate::BackendError::new(format!("Failed to lock connection: {}", e))
            })?;

            create_content_table(
                &mut conn,
                table_name,
                template,
                Some(context.narrative_name),
                Some(&context.narrative_metadata.description),
            )?;
        }

        // Extract JSON from response
        let json_str = extract_json(&context.execution.response)?;

        tracing::debug!(json_length = json_str.len(), "Extracted JSON from response");

        // Parse JSON - could be single object or array
        let items: Vec<JsonValue> = if json_str.trim().starts_with('[') {
            parse_json(&json_str)?
        } else {
            vec![parse_json(&json_str)?]
        };

        tracing::info!(count = items.len(), "Parsed JSON items for insertion");

        // Insert each item
        for (idx, item) in items.iter().enumerate() {
            tracing::debug!(
                index = idx,
                act = %context.execution.act_name,
                "Inserting content item"
            );

            self.insert_content(
                table_name,
                item,
                context.narrative_name,
                &context.execution.act_name,
                context.execution.model.as_deref(),
            )?;
        }

        tracing::info!(
            act = %context.execution.act_name,
            table = %table_name,
            count = items.len(),
            "Content generation completed successfully"
        );

        Ok(())
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        // Process if narration has a template field
        context.narrative_metadata.template.is_some()
    }

    fn name(&self) -> &str {
        "ContentGenerationProcessor"
    }
}

/// Convert a JSON value to SQL literal format.
#[cfg(feature = "database")]
/// Convert a JSON value to SQL literal with proper type casting.
///
/// Handles type conversions based on PostgreSQL column type:
/// - text[] (PostgreSQL arrays) from JSON arrays
/// - jsonb from JSON objects or arrays (when column is jsonb)
/// - Primitives (string, number, bool, null)
fn json_value_to_sql(value: &JsonValue, col_type: &str) -> String {
    match value {
        JsonValue::Null => "NULL".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => format!("'{}'", s.replace('\'', "''")), // SQL escape
        JsonValue::Array(arr) => {
            // Check if target column is a PostgreSQL array type
            if col_type == "ARRAY" || col_type.contains("[]") {
                // Format as PostgreSQL array literal: ARRAY['val1', 'val2']
                let elements: Vec<String> = arr
                    .iter()
                    .map(|v| match v {
                        JsonValue::String(s) => format!("'{}'", s.replace('\'', "''")),
                        JsonValue::Number(n) => n.to_string(),
                        JsonValue::Bool(b) => b.to_string(),
                        JsonValue::Null => "NULL".to_string(),
                        _ => format!("'{}'", v.to_string().replace('\'', "''")),
                    })
                    .collect();
                format!("ARRAY[{}]", elements.join(", "))
            } else {
                // Store as JSONB
                format!("'{}'::jsonb", value.to_string().replace('\'', "''"))
            }
        }
        JsonValue::Object(_) => {
            // Objects always stored as JSONB
            format!("'{}'::jsonb", value.to_string().replace('\'', "''"))
        }
    }
}
