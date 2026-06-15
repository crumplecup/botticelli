//! Storage actor for asynchronous content operations using Ractor.
//!
//! Provides an actor-based abstraction for content storage operations,
//! dispatching messages to an `Arc<dyn BotStorage>` backend. Schema
//! creation (template/inference) is a no-op for key-value backends —
//! tables are implicit collections keyed by `table_name`.

use async_trait::async_trait;
use botticelli_error::{BackendError, BotticelliError, BotticelliResult};
use botticelli_interface::{BotStorage, ContentGenerationRecord, ContentRecord};
use chrono::Utc;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// Storage actor handling all content persistence operations.
pub struct StorageActor {
    storage: Arc<dyn BotStorage>,
}

impl StorageActor {
    /// Create a new storage actor with a backing storage implementation.
    pub fn new(storage: Arc<dyn BotStorage>) -> Self {
        Self { storage }
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
    ///
    /// For key-value backends this is a no-op; tables are implicit.
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
    ///
    /// For key-value backends this is a no-op; schema is inferred from stored JSON.
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
        /// Final status: "success" or "failed".
        status: String,
        /// Optional error message if status is "failed".
        error_message: Option<String>,
        /// Reply port for RPC response.
        reply: RpcReplyPort<BotticelliResult<()>>,
    },
}

/// Active generation tracker — maps `table_name` to `generation_id`.
pub struct StorageActorState {
    active_generations: HashMap<String, String>,
}

#[async_trait]
impl Actor for StorageActor {
    type Msg = StorageMessage;
    type State = StorageActorState;
    type Arguments = Arc<dyn BotStorage>;

    #[instrument(skip_all)]
    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("StorageActor started");
        Ok(StorageActorState {
            active_generations: HashMap::new(),
        })
    }

    #[instrument(skip_all)]
    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        tracing::info!("StorageActor stopped");
        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            StorageMessage::StartGeneration {
                table_name,
                narrative_file,
                narrative_name,
                reply,
            } => {
                let result = self
                    .handle_start_generation(&table_name, narrative_file, narrative_name, state)
                    .await;
                let _ = reply.send(result);
            }
            StorageMessage::CreateTableFromTemplate {
                table_name,
                template,
                narrative_name: _,
                description: _,
                reply,
            } => {
                tracing::debug!(
                    table = %table_name,
                    template = %template,
                    "CreateTableFromTemplate is a no-op for key-value backends"
                );
                let _ = reply.send(Ok(()));
            }
            StorageMessage::CreateTableFromInference {
                table_name,
                json_sample: _,
                narrative_name: _,
                description: _,
                reply,
            } => {
                tracing::debug!(
                    table = %table_name,
                    "CreateTableFromInference is a no-op for key-value backends"
                );
                let _ = reply.send(Ok(()));
            }
            StorageMessage::InsertContent {
                table_name,
                json_data,
                narrative_name,
                act_name,
                model,
                reply,
            } => {
                let result = self
                    .handle_insert_content(table_name, json_data, narrative_name, act_name, model)
                    .await;
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
                let result = self
                    .handle_complete_generation(
                        &table_name,
                        row_count,
                        duration_ms,
                        status,
                        error_message,
                        state,
                    )
                    .await;
                let _ = reply.send(result);
            }
        }
        Ok(())
    }
}

impl StorageActor {
    #[instrument(skip(self, state), fields(%table_name, %narrative_name))]
    async fn handle_start_generation(
        &self,
        table_name: &str,
        narrative_file: String,
        narrative_name: String,
        state: &mut StorageActorState,
    ) -> BotticelliResult<()> {
        let id = Uuid::new_v4().to_string();
        let record = ContentGenerationRecord {
            id: id.clone(),
            table_name: table_name.to_string(),
            narrative_file,
            narrative_name,
            generated_at: Utc::now(),
            completed_at: None,
            row_count: None,
            generation_duration_ms: None,
            status: "running".to_string(),
            error_message: None,
            created_by: None,
        };

        self.storage
            .save_content_generation(&record)
            .await
            .map_err(|e| BotticelliError::from(BackendError::new(format!("{e}"))))?;

        state.active_generations.insert(table_name.to_string(), id);
        tracing::info!(table = %table_name, "Started tracking content generation");
        Ok(())
    }

    #[instrument(skip(self, json_data), fields(%table_name, %narrative_name, %act_name))]
    async fn handle_insert_content(
        &self,
        table_name: String,
        json_data: JsonValue,
        narrative_name: String,
        act_name: String,
        model: Option<String>,
    ) -> BotticelliResult<()> {
        let record = ContentRecord {
            id: Uuid::new_v4().to_string(),
            table_name: table_name.clone(),
            content_json: json_data,
            created_at: Utc::now(),
        };

        self.storage
            .save_content(&record)
            .await
            .map_err(|e| BotticelliError::from(BackendError::new(format!("{e}"))))?;

        tracing::debug!(
            table = %table_name,
            act = %act_name,
            model = ?model,
            narrative = %narrative_name,
            "Content inserted"
        );
        Ok(())
    }

    #[instrument(skip(self, state), fields(%table_name, %status))]
    async fn handle_complete_generation(
        &self,
        table_name: &str,
        row_count: Option<i32>,
        duration_ms: i32,
        status: String,
        error_message: Option<String>,
        state: &mut StorageActorState,
    ) -> BotticelliResult<()> {
        let id = match state.active_generations.remove(table_name) {
            Some(id) => id,
            None => {
                tracing::warn!(table = %table_name, "No active generation found for table");
                return Ok(());
            }
        };

        let mut records = self
            .storage
            .list_content_generations_by_table(table_name, 10)
            .await
            .map_err(|e| BotticelliError::from(BackendError::new(format!("{e}"))))?;

        let Some(mut record) = records.drain(..).find(|r| r.id == id) else {
            tracing::warn!(table = %table_name, id = %id, "Generation record not found");
            return Ok(());
        };

        record.completed_at = Some(Utc::now());
        record.row_count = row_count;
        record.generation_duration_ms = Some(duration_ms);
        record.status = status.clone();
        record.error_message = error_message;

        self.storage
            .save_content_generation(&record)
            .await
            .map_err(|e| BotticelliError::from(BackendError::new(format!("{e}"))))?;

        tracing::info!(
            table = %table_name,
            row_count = ?row_count,
            duration_ms = duration_ms,
            status = %status,
            "Content generation completed"
        );
        Ok(())
    }
}
