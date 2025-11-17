//! Content generation processor for creating reviewable content.
//!
//! This processor detects narratives with a `template` field and generates
//! content into custom tables based on Discord schema templates, OR infers
//! schema automatically from JSON responses when no template is provided.

#[cfg(feature = "database")]
use crate::{
    ActProcessor, BotticelliResult, ContentGenerationRepository, NewContentGenerationRow,
    PgConnection, PostgresContentGenerationRepository, ProcessorContext,
    UpdateContentGenerationRow, create_content_table, create_inferred_table, extract_json,
    infer_schema, parse_json,
};
#[cfg(feature = "database")]
use async_trait::async_trait;
#[cfg(feature = "database")]
use chrono::Utc;
#[cfg(feature = "database")]
use diesel::prelude::*;
#[cfg(feature = "database")]
use serde_json::Value as JsonValue;
#[cfg(feature = "database")]
use std::sync::{Arc, Mutex};

/// Content generation processing mode
#[cfg(feature = "database")]
#[derive(Debug, Clone, PartialEq)]
enum ProcessingMode {
    /// Use an explicit template to define table schema
    Template(String),
    /// Infer schema automatically from JSON response
    Inference,
}

/// Processor for content generation into custom tables.
///
/// Detects narratives with a `template` field and:
/// 1. Creates a custom table if it doesn't exist (based on template schema)
/// 2. Tracks generation start in content_generations table
/// 3. Extracts JSON from LLM responses
/// 4. Inserts generated content with metadata columns
/// 5. Updates tracking record with success/failure
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
    ) -> BotticelliResult<()> {
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
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        let table_name = &context.narrative_metadata.name;

        // Determine processing mode: Template or Inference
        let processing_mode = if let Some(template) = &context.narrative_metadata.template {
            ProcessingMode::Template(template.clone())
        } else {
            ProcessingMode::Inference
        };

        tracing::info!(
            act = %context.execution.act_name,
            table = %table_name,
            mode = ?processing_mode,
            "Processing content generation"
        );

        let start_time = std::time::Instant::now();

        // Track generation start
        {
            let mut conn = self.connection.lock().map_err(|e| {
                crate::BackendError::new(format!("Failed to lock connection: {}", e))
            })?;

            let mut repo = PostgresContentGenerationRepository::new(&mut conn);

            // Try to start tracking (ignore unique constraint violations if already exists)
            let new_gen = NewContentGenerationRow {
                table_name: table_name.clone(),
                narrative_file: format!("{} (from processor)", context.narrative_name),
                narrative_name: context.narrative_name.to_string(),
                status: "running".to_string(),
                created_by: None,
            };

            if let Err(e) = repo.start_generation(new_gen) {
                tracing::debug!(
                    error = %e,
                    table = %table_name,
                    "Could not start tracking (may already exist, continuing)"
                );
            } else {
                tracing::info!(table = %table_name, "Started tracking content generation");
            }
        }

        // Execute content generation
        let generation_result: Result<usize, crate::BotticelliError> = (|| {
            // Extract JSON from response first (needed for both modes)
            let json_str = extract_json(&context.execution.response)?;

            tracing::debug!(json_length = json_str.len(), "Extracted JSON from response");

            // Parse JSON - could be single object or array
            let parsed_json: JsonValue = parse_json(&json_str)?;

            let items: Vec<JsonValue> = if parsed_json.is_array() {
                parsed_json.as_array().unwrap().to_vec()
            } else {
                vec![parsed_json.clone()]
            };

            // Create table based on processing mode
            {
                let mut conn = self.connection.lock().map_err(|e| {
                    crate::BackendError::new(format!("Failed to lock connection: {}", e))
                })?;

                match &processing_mode {
                    ProcessingMode::Template(template) => {
                        tracing::debug!(template = %template, "Creating table from template");

                        create_content_table(
                            &mut conn,
                            table_name,
                            template,
                            Some(context.narrative_name),
                            Some(&context.narrative_metadata.description),
                        )?;
                    }
                    ProcessingMode::Inference => {
                        tracing::debug!("Inferring schema from JSON response");

                        // Infer schema from parsed JSON
                        let schema = infer_schema(&parsed_json)?;

                        tracing::info!(
                            field_count = schema.field_count(),
                            "Inferred schema from JSON"
                        );

                        create_inferred_table(
                            &mut conn,
                            table_name,
                            &schema,
                            Some(context.narrative_name),
                            Some(&context.narrative_metadata.description),
                        )?;
                    }
                }
            }

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

            Ok(items.len())
        })();

        // Update tracking record with result
        let duration_ms = start_time.elapsed().as_millis() as i32;

        {
            let mut conn = self.connection.lock().map_err(|e| {
                crate::BackendError::new(format!("Failed to lock connection: {}", e))
            })?;

            let mut repo = PostgresContentGenerationRepository::new(&mut conn);

            match generation_result {
                Ok(row_count) => {
                    let update = UpdateContentGenerationRow {
                        completed_at: Some(Utc::now()),
                        row_count: Some(row_count as i32),
                        generation_duration_ms: Some(duration_ms),
                        status: Some("success".to_string()),
                        error_message: None,
                    };

                    if let Err(e) = repo.complete_generation(table_name, update) {
                        tracing::warn!(
                            error = %e,
                            table = %table_name,
                            "Failed to update tracking record"
                        );
                    } else {
                        tracing::info!(
                            table = %table_name,
                            row_count,
                            duration_ms,
                            "Updated tracking: generation successful"
                        );
                    }
                }
                Err(ref e) => {
                    let update = UpdateContentGenerationRow {
                        completed_at: Some(Utc::now()),
                        row_count: None,
                        generation_duration_ms: Some(duration_ms),
                        status: Some("failed".to_string()),
                        error_message: Some(e.to_string()),
                    };

                    if let Err(update_err) = repo.complete_generation(table_name, update) {
                        tracing::warn!(
                            error = %update_err,
                            table = %table_name,
                            "Failed to update tracking record with failure"
                        );
                    } else {
                        tracing::info!(
                            table = %table_name,
                            error = %e,
                            duration_ms,
                            "Updated tracking: generation failed"
                        );
                    }
                }
            }
        }

        // Return the original result
        generation_result.map(|row_count| {
            tracing::info!(
                act = %context.execution.act_name,
                table = %table_name,
                count = row_count,
                "Content generation completed successfully"
            );
        })
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        // Don't process if user explicitly opted out
        if context.narrative_metadata.skip_content_generation {
            return false;
        }

        // Otherwise, process (with template OR inference mode)
        true
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
