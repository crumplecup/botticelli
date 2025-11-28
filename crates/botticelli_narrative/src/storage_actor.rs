//! Storage actor for asynchronous table operations using Ractor.
//!
//! This module provides an actor-based abstraction for content storage,
//! handling table creation, schema inference, and row insertion through
//! an asynchronous message-passing interface.

use async_trait::async_trait;
use botticelli_database::{
    ContentGenerationRepository, NewContentGenerationRow, PostgresContentGenerationRepository,
    UpdateContentGenerationRow, create_content_table, create_inferred_table, infer_schema,
    reflect_table_schema,
};
use botticelli_error::BotticelliResult;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Storage actor handling all database operations for content generation.
pub struct StorageActor {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl StorageActor {
    /// Create a new storage actor with a connection pool.
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    /// Get a connection from the pool.
    fn get_conn(
        &self,
    ) -> BotticelliResult<diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>>> {
        Ok(self.pool.get().map_err(|e| {
            botticelli_error::BackendError::new(format!(
                "Failed to get connection from pool: {}",
                e
            ))
        })?)
    }
}

/// Messages that the StorageActor can handle.
#[derive(Debug)]
pub enum StorageMessage {
    /// Start tracking a content generation.
    StartGeneration {
        /// Target table name for content storage.
        table_name: String,
        /// Path to the narrative file.
        narrative_file: String,
        /// Name of the narrative being executed.
        narrative_name: String,
        /// Reply port for RPC response.
        reply: RpcReplyPort<BotticelliResult<()>>,
    },
    /// Create a table from a template.
    CreateTableFromTemplate {
        /// Target table name to create.
        table_name: String,
        /// Template table name to copy schema from.
        template: String,
        /// Optional narrative name for metadata.
        narrative_name: Option<String>,
        /// Optional description for the table.
        description: Option<String>,
        /// Reply port for RPC response.
        reply: RpcReplyPort<BotticelliResult<()>>,
    },
    /// Create a table with inferred schema.
    CreateTableFromInference {
        /// Target table name to create.
        table_name: String,
        /// Sample JSON data for schema inference.
        json_sample: JsonValue,
        /// Optional narrative name for metadata.
        narrative_name: Option<String>,
        /// Optional description for the table.
        description: Option<String>,
        /// Reply port for RPC response.
        reply: RpcReplyPort<BotticelliResult<()>>,
    },
    /// Insert content into a table.
    InsertContent {
        /// Target table name for insertion.
        table_name: String,
        /// JSON data to insert.
        json_data: JsonValue,
        /// Name of the narrative generating content.
        narrative_name: String,
        /// Name of the act generating content.
        act_name: String,
        /// Optional model name used for generation.
        model: Option<String>,
        /// Reply port for RPC response.
        reply: RpcReplyPort<BotticelliResult<()>>,
    },
    /// Complete a content generation.
    CompleteGeneration {
        /// Target table name.
        table_name: String,
        /// Number of rows generated.
        row_count: Option<i32>,
        /// Duration in milliseconds.
        duration_ms: i32,
        /// Final status (success/failed).
        status: String,
        /// Optional error message if failed.
        error_message: Option<String>,
        /// Reply port for RPC response.
        reply: RpcReplyPort<BotticelliResult<()>>,
    },
}

/// State is unit type since all state is in the actor struct
pub struct StorageActorState;

#[async_trait]
impl Actor for StorageActor {
    type Msg = StorageMessage;
    type State = StorageActorState;
    type Arguments = Pool<ConnectionManager<PgConnection>>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("StorageActor started");
        Ok(StorageActorState)
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        tracing::info!("StorageActor stopped");
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            StorageMessage::StartGeneration {
                table_name,
                narrative_file,
                narrative_name,
                reply,
            } => {
                let result =
                    self.handle_start_generation(table_name, narrative_file, narrative_name);
                let _ = reply.send(result);
            }
            StorageMessage::CreateTableFromTemplate {
                table_name,
                template,
                narrative_name,
                description,
                reply,
            } => {
                let result = self.handle_create_from_template(
                    table_name,
                    template,
                    narrative_name,
                    description,
                );
                let _ = reply.send(result);
            }
            StorageMessage::CreateTableFromInference {
                table_name,
                json_sample,
                narrative_name,
                description,
                reply,
            } => {
                let result = self.handle_create_from_inference(
                    table_name,
                    json_sample,
                    narrative_name,
                    description,
                );
                let _ = reply.send(result);
            }
            StorageMessage::InsertContent {
                table_name,
                json_data,
                narrative_name,
                act_name,
                model,
                reply,
            } => {
                let result = self.handle_insert_content(
                    table_name,
                    json_data,
                    narrative_name,
                    act_name,
                    model,
                );
                let _ = reply.send(result);
            }
            StorageMessage::CompleteGeneration {
                table_name,
                row_count,
                duration_ms,
                status,
                error_message,
                reply,
            } => {
                let result = self.handle_complete_generation(
                    table_name,
                    row_count,
                    duration_ms,
                    status,
                    error_message,
                );
                let _ = reply.send(result);
            }
        }
        Ok(())
    }
}

impl StorageActor {
    fn handle_start_generation(
        &self,
        table_name: String,
        narrative_file: String,
        narrative_name: String,
    ) -> BotticelliResult<()> {
        let mut conn = self.get_conn()?;
        let mut repo = PostgresContentGenerationRepository::new(&mut conn);

        let new_gen = NewContentGenerationRow {
            table_name: table_name.clone(),
            narrative_file,
            narrative_name,
            status: "running".to_string(),
            created_by: None,
        };

        repo.start_generation(new_gen).map_err(|e| {
            tracing::debug!(
                error = %e,
                table = %table_name,
                "Could not start tracking (may already exist)"
            );
            e
        })?;

        tracing::info!(table = %table_name, "Started tracking content generation");
        Ok(())
    }

    fn handle_create_from_template(
        &self,
        table_name: String,
        template: String,
        narrative_name: Option<String>,
        description: Option<String>,
    ) -> BotticelliResult<()> {
        let mut conn = self.get_conn()?;

        tracing::debug!(
            template = %template,
            table = %table_name,
            "Creating table from template"
        );

        create_content_table(
            &mut conn,
            &table_name,
            &template,
            narrative_name.as_deref(),
            description.as_deref(),
        )?;

        tracing::info!(table = %table_name, "Table created from template");
        Ok(())
    }

    fn handle_create_from_inference(
        &self,
        table_name: String,
        json_sample: JsonValue,
        narrative_name: Option<String>,
        description: Option<String>,
    ) -> BotticelliResult<()> {
        let mut conn = self.get_conn()?;

        tracing::debug!(table = %table_name, "Inferring schema from JSON");

        let schema = infer_schema(&json_sample)?;

        tracing::info!(
            field_count = schema.field_count(),
            table = %table_name,
            "Inferred schema from JSON"
        );

        create_inferred_table(
            &mut conn,
            &table_name,
            &schema,
            narrative_name.as_deref(),
            description.as_deref(),
        )?;

        tracing::info!(table = %table_name, "Inferred table created successfully");
        Ok(())
    }

    fn handle_insert_content(
        &self,
        table_name: String,
        json_data: JsonValue,
        narrative_name: String,
        act_name: String,
        model: Option<String>,
    ) -> BotticelliResult<()> {
        let mut conn = self.get_conn()?;

        // Query schema to get column types and constraints
        let schema = reflect_table_schema(&mut conn, &table_name)?;
        let column_types: std::collections::HashMap<_, _> = schema
            .columns
            .iter()
            .map(|col| (col.name.as_str(), col.data_type.as_str()))
            .collect();

        // Track required (NOT NULL) columns, excluding auto-generated ones
        let required_columns: Vec<&str> = schema
            .columns
            .iter()
            .filter(|col| col.is_nullable == "NO" && col.column_default.is_none())
            .map(|col| col.name.as_str())
            .collect();

        // Build INSERT statement dynamically
        let obj = json_data
            .as_object()
            .ok_or_else(|| botticelli_error::BackendError::new("JSON must be an object"))?;

        let mut columns = Vec::new();
        let mut values = Vec::new();

        // Add content fields from JSON (with fuzzy matching)
        let mut provided_columns = std::collections::HashSet::new();
        for (key, value) in obj {
            if let Some((col_name, col_type)) = find_column_match(key, &column_types) {
                columns.push(col_name.to_string());
                values.push(json_value_to_sql(value, col_type));
                provided_columns.insert(col_name);
            } else {
                tracing::debug!(
                    field = %key,
                    table = %table_name,
                    "Ignoring extra JSON field not in table schema"
                );
            }
        }

        // Validate required fields before INSERT
        // Per JSON_SCHEMA_MISMATCH_STRATEGY: Allow missing fields (will be NULL)
        let missing_required: Vec<&str> = required_columns
            .iter()
            .filter(|&&col| {
                !provided_columns.contains(col)
                    && col != "source_narrative"
                    && col != "source_act"
                    && col != "model"
            })
            .copied()
            .collect();

        if !missing_required.is_empty() {
            tracing::warn!(
                table = %table_name,
                missing_fields = ?missing_required,
                provided_fields = ?provided_columns,
                "JSON missing some table fields - will use NULL for missing fields"
            );
            // Add NULL values for missing required fields
            for &missing_col in &missing_required {
                columns.push(missing_col.to_string());
                values.push("NULL".to_string());
            }
        }

        // Add metadata columns
        columns.push("source_narrative".to_string());
        values.push(format!("'{}'", narrative_name));

        columns.push("source_act".to_string());
        values.push(format!("'{}'", act_name));

        if let Some(m) = &model {
            columns.push("generation_model".to_string());
            values.push(format!("'{}'", m));
        }

        columns.push("generated_at".to_string());
        values.push("NOW()".to_string());

        // Execute INSERT
        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            values.join(", ")
        );

        tracing::debug!(sql = %query, "Executing INSERT");

        diesel::sql_query(&query).execute(&mut conn).map_err(|e| {
            botticelli_error::BackendError::new(format!("Failed to insert content: {}", e))
        })?;

        tracing::debug!(
            table = %table_name,
            act = %act_name,
            "Content inserted successfully"
        );

        Ok(())
    }

    fn handle_complete_generation(
        &self,
        table_name: String,
        row_count: Option<i32>,
        duration_ms: i32,
        status: String,
        error_message: Option<String>,
    ) -> BotticelliResult<()> {
        let mut conn = self.get_conn()?;
        let mut repo = PostgresContentGenerationRepository::new(&mut conn);

        let update = UpdateContentGenerationRow {
            completed_at: Some(Utc::now()),
            row_count,
            generation_duration_ms: Some(duration_ms),
            status: Some(status.clone()),
            error_message: error_message.clone(),
        };

        repo.complete_generation(&table_name, update).map_err(|e| {
            tracing::warn!(
                error = %e,
                table = %table_name,
                "Failed to update tracking record"
            );
            e
        })?;

        tracing::info!(
            table = %table_name,
            row_count = ?row_count,
            duration_ms = duration_ms,
            status = %status,
            "Updated tracking: generation complete"
        );

        Ok(())
    }
}

/// Convert a JSON value to SQL literal string.
/// Converts JSON field name to potential database column name variations
fn field_name_variations(field: &str) -> Vec<String> {
    vec![
        field.to_string(),    // exact match
        field.to_lowercase(), // lowercase
        to_snake_case(field), // snake_case
        to_camel_case(field), // camelCase
    ]
}

/// Convert string to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

/// Convert string to camelCase
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Find matching column name using fuzzy matching
fn find_column_match<'a>(
    field: &str,
    column_types: &'a HashMap<&str, &str>,
) -> Option<(&'a str, &'a str)> {
    for variant in field_name_variations(field) {
        if let Some(&col_type) = column_types.get(variant.as_str()) {
            if variant != field {
                tracing::debug!(
                    json_field = %field,
                    db_column = %variant,
                    "Fuzzy matched field name"
                );
            }
            return Some((
                column_types
                    .iter()
                    .find(|(k, _)| **k == variant)
                    .map(|(k, _)| *k)
                    .unwrap(),
                col_type,
            ));
        }
    }
    None
}

/// Format table schema as human-readable string for LLM prompts
/// Formats schema for LLM prompts (reserved for Phase 2 improved prompts)
#[allow(dead_code)]
fn format_schema_for_prompt(schema: &botticelli_database::TableSchema) -> String {
    let mut result = String::from("{\n");

    for col in &schema.columns {
        // Skip auto-generated metadata columns
        if col.name == "source_narrative" || col.name == "source_act" || col.name == "model" {
            continue;
        }

        let required = if col.is_nullable == "NO" && col.column_default.is_none() {
            "required"
        } else {
            "optional"
        };

        let type_hint = match col.data_type.as_str() {
            "integer" | "bigint" | "smallint" => "integer",
            "real" | "double precision" | "numeric" => "number",
            "boolean" => "boolean",
            "text" | "varchar" | "char" => "string",
            "jsonb" | "json" => "object or array",
            other => other,
        };

        result.push_str(&format!(
            "  \"{}\": {} ({}),\n",
            col.name, type_hint, required
        ));
    }

    result.push_str("}\n");
    result
}

/// Converts a JSON value to SQL literal based on column type with best-effort coercion
fn json_value_to_sql(value: &JsonValue, col_type: &str) -> String {
    use serde_json::Value;

    let col_type_lower = col_type.to_lowercase();

    // Handle PostgreSQL array types (e.g., _text = text[], _int4 = integer[])
    if col_type_lower.starts_with('_') {
        match value {
            Value::Array(arr) => {
                // Convert JSON array to PostgreSQL ARRAY constructor
                let elements: Vec<String> = arr
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "NULL".to_string(),
                        _ => format!("'{}'", v.to_string().replace('\'', "''")),
                    })
                    .collect();
                return format!("ARRAY[{}]", elements.join(", "));
            }
            Value::Null => return "NULL".to_string(),
            _ => {
                tracing::warn!(
                    value = ?value,
                    col_type = %col_type,
                    "Expected array for array column type, using empty array"
                );
                return "ARRAY[]".to_string();
            }
        }
    }

    match col_type_lower.as_str() {
        "integer" | "int" | "int4" | "bigint" | "int8" | "smallint" | "int2" => {
            match value {
                Value::Number(n) => n.to_string(),
                Value::String(s) => {
                    // Try to parse string as integer
                    if let Ok(i) = s.parse::<i64>() {
                        tracing::debug!(value = %s, parsed = i, "Coerced string to integer");
                        i.to_string()
                    } else {
                        tracing::warn!(value = %s, col_type = %col_type, "Failed to coerce string to integer, using NULL");
                        "NULL".to_string()
                    }
                }
                Value::Bool(b) => (*b as i32).to_string(),
                Value::Null => "NULL".to_string(),
                _ => {
                    tracing::warn!(value = ?value, col_type = %col_type, "Cannot coerce to integer, using NULL");
                    "NULL".to_string()
                }
            }
        }
        "real" | "float4" | "double precision" | "float8" | "numeric" | "decimal" => match value {
            Value::Number(n) => n.to_string(),
            Value::String(s) => {
                if let Ok(f) = s.parse::<f64>() {
                    tracing::debug!(value = %s, parsed = f, "Coerced string to float");
                    f.to_string()
                } else {
                    tracing::warn!(value = %s, col_type = %col_type, "Failed to coerce string to float, using NULL");
                    "NULL".to_string()
                }
            }
            Value::Bool(b) => (*b as i32 as f64).to_string(),
            Value::Null => "NULL".to_string(),
            _ => {
                tracing::warn!(value = ?value, col_type = %col_type, "Cannot coerce to float, using NULL");
                "NULL".to_string()
            }
        },
        "boolean" | "bool" => match value {
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    (i != 0).to_string()
                } else {
                    "true".to_string()
                }
            }
            Value::String(s) => {
                let lower = s.to_lowercase();
                match lower.as_str() {
                    "true" | "t" | "yes" | "y" | "1" => "true".to_string(),
                    "false" | "f" | "no" | "n" | "0" => "false".to_string(),
                    _ => {
                        tracing::warn!(value = %s, "Cannot coerce string to boolean, using false");
                        "false".to_string()
                    }
                }
            }
            Value::Null => "NULL".to_string(),
            _ => "false".to_string(),
        },
        "jsonb" | "json" => {
            // Store complex types as JSONB
            match value {
                Value::Null => "NULL".to_string(),
                Value::String(s) => format!("'{}'::jsonb", s.replace('\'', "''")),
                Value::Array(_) | Value::Object(_) => {
                    format!("'{}'::jsonb", value.to_string().replace('\'', "''"))
                }
                _ => {
                    // Wrap primitives in JSON
                    format!("'{}'::jsonb", value.to_string().replace('\'', "''"))
                }
            }
        }
        "text" | "varchar" | "char" => {
            // Text types: convert to text
            match value {
                Value::Null => "NULL".to_string(),
                Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                Value::Bool(b) => format!("'{}'", b),
                Value::Number(n) => format!("'{}'", n),
                Value::Array(_) | Value::Object(_) => {
                    // Serialize complex types to JSON string for text columns
                    tracing::debug!("Storing complex type as JSON string in text column");
                    format!("'{}'", value.to_string().replace('\'', "''"))
                }
            }
        }
        _ => {
            // Unknown types: default to text conversion
            match value {
                Value::Null => "NULL".to_string(),
                Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                Value::Bool(b) => format!("'{}'", b),
                Value::Number(n) => format!("'{}'", n),
                Value::Array(_) | Value::Object(_) => {
                    tracing::debug!(col_type, "Unknown column type, storing as JSON string");
                    format!("'{}'", value.to_string().replace('\'', "''"))
                }
            }
        }
    }
}
