//! PostgreSQL implementation of NarrativeRepository.

use crate::narrative_conversions::{
    act_execution_to_new_row, execution_to_new_row, input_to_new_row, rows_to_act_execution,
    rows_to_narrative_execution, status_to_string, string_to_status,
};
use crate::schema::{act_executions, act_inputs, narrative_executions};
use crate::{ActExecutionRow, ActInputRow, NarrativeExecutionRow};

use botticelli_core::{ExecutionFilter, ExecutionStatus, ExecutionSummary, NarrativeExecution};
use botticelli_error::{BackendError, BotticelliError, BotticelliResult};
use botticelli_interface::{MediaStorage, NarrativeRepository, NarrativeStorageOperations};
use botticelli_storage::{MediaMetadata, MediaReference};

use async_trait::async_trait;
use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// PostgreSQL implementation of NarrativeRepository using Diesel ORM.
///
/// This repository stores narrative executions in a PostgreSQL database using
/// three tables: narrative_executions, act_executions, and act_inputs.
///
/// # Example
/// ```no_run
/// use botticelli_database::{PostgresNarrativeRepository, establish_connection};
/// use botticelli_interface::NarrativeRepository;
/// use botticelli_storage::FileSystemStorage;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut conn = establish_connection()?;
///     let storage = Arc::new(FileSystemStorage::new("/var/botticelli/media")?);
///     let repo = PostgresNarrativeRepository::new(conn, storage);
///     // Use repo.save_execution(), load_execution(), etc.
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct PostgresNarrativeRepository {
    /// Database connection wrapped in Arc<Mutex> for async safety.
    ///
    /// Note: This is a simple implementation. For production use, consider using
    /// a connection pool like r2d2 or deadpool.
    conn: Arc<Mutex<PgConnection>>,
    /// Media storage backend for binary content
    storage: Arc<
        dyn MediaStorage<
                Error = BotticelliError,
                Metadata = MediaMetadata,
                Reference = MediaReference,
            >,
    >,
}

impl PostgresNarrativeRepository {
    /// Create a new PostgreSQL narrative repository.
    ///
    /// # Arguments
    /// * `conn` - A PostgreSQL connection
    /// * `storage` - Media storage backend
    ///
    /// # Note
    /// The connection is wrapped in Arc<Mutex> to allow async access.
    /// For better performance with concurrent access, consider using a connection pool.
    #[tracing::instrument(skip(conn, storage))]
    pub fn new(
        conn: PgConnection,
        storage: Arc<
            dyn MediaStorage<
                    Error = BotticelliError,
                    Metadata = MediaMetadata,
                    Reference = MediaReference,
                >,
        >,
    ) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
            storage,
        }
    }

    /// Create a repository from an Arc<Mutex<PgConnection>> (for sharing connections).
    #[tracing::instrument(skip(conn, storage))]
    pub fn from_arc(
        conn: Arc<Mutex<PgConnection>>,
        storage: Arc<
            dyn MediaStorage<
                    Error = BotticelliError,
                    Metadata = MediaMetadata,
                    Reference = MediaReference,
                >,
        >,
    ) -> Self {
        Self { conn, storage }
    }
}

#[async_trait]
impl NarrativeRepository for PostgresNarrativeRepository {
    type Error = BotticelliError;
    type MediaMetadata = MediaMetadata;
    type MediaReference = MediaReference;
    type Execution = NarrativeExecution;
    type Filter = ExecutionFilter;
    type Summary = ExecutionSummary;
    type Status = ExecutionStatus;

    #[tracing::instrument(skip(self, execution), fields(act_count = execution.act_executions().len()))]
    async fn save_execution(&self, execution: &Self::Execution) -> Result<i32, Self::Error> {
        tracing::debug!("Saving narrative execution");
        let mut conn = self.conn.lock().await;

        // Use a transaction for atomicity
        let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // Insert narrative_execution
            let new_execution = execution_to_new_row(execution, ExecutionStatus::Completed);
            let execution_row: NarrativeExecutionRow =
                diesel::insert_into(narrative_executions::table)
                    .values(&new_execution)
                    .get_result(conn)?;

            let execution_id = execution_row.id;
            tracing::debug!(execution_id, "Inserted narrative execution");

            // Insert all acts
            for act in execution.act_executions() {
                let new_act = act_execution_to_new_row(act, execution_id);
                let act_row: ActExecutionRow = diesel::insert_into(act_executions::table)
                    .values(&new_act)
                    .get_result(conn)?;

                // Insert all inputs for this act
                for (order, input) in act.inputs().iter().enumerate() {
                    let new_input = match input_to_new_row(input, act_row.id, order) {
                        Ok(row) => row,
                        Err(_) => return Err(diesel::result::Error::RollbackTransaction),
                    };
                    diesel::insert_into(act_inputs::table)
                        .values(&new_input)
                        .execute(conn)?;
                }
            }

            Ok(execution_id)
        });

        match result {
            Ok(id) => {
                tracing::info!(execution_id = id, "Saved narrative execution");
                Ok(id)
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to save execution");
                Err(BotticelliError::from(BackendError::new(format!("Transaction failed: {}", e))))
            }
        }
    }

    #[tracing::instrument(skip(self), fields(id))]
    async fn load_execution(&self, id: i32) -> BotticelliResult<NarrativeExecution> {
        tracing::debug!("Loading narrative execution");
        let mut conn = self.conn.lock().await;

        // Load the narrative execution
        let execution_row: NarrativeExecutionRow = narrative_executions::table
            .find(id)
            .first(&mut *conn)
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to load narrative execution");
                BotticelliError::from(BackendError::new(format!(
                    "Failed to load narrative execution {}: {}",
                    id, e
                )))
            })?;

        // Load all acts for this execution
        let act_rows: Vec<ActExecutionRow> = ActExecutionRow::belonging_to(&execution_row)
            .order(act_executions::sequence_number.asc())
            .load(&mut *conn)
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to load act executions");
                BotticelliError::from(BackendError::new(format!(
                    "Failed to load act executions: {}",
                    e
                )))
            })?;

        tracing::debug!(act_count = act_rows.len(), "Loaded act executions");

        // Load all inputs for all acts
        let input_rows: Vec<ActInputRow> = ActInputRow::belonging_to(&act_rows)
            .load(&mut *conn)
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to load act inputs");
                BotticelliError::from(BackendError::new(format!(
                    "Failed to load act inputs: {}",
                    e
                )))
            })?;

        tracing::debug!(input_count = input_rows.len(), "Loaded act inputs");

        // Group inputs by act
        let inputs_by_act =
            input_rows
                .into_iter()
                .fold(std::collections::HashMap::new(), |mut acc, input| {
                    acc.entry(input.act_execution_id)
                        .or_insert_with(Vec::new)
                        .push(input);
                    acc
                });

        // Reconstruct ActExecutions
        let mut act_executions = Vec::new();
        for act_row in act_rows {
            let inputs = inputs_by_act.get(&act_row.id).cloned().unwrap_or_default();
            let act = rows_to_act_execution(act_row, inputs)?;
            act_executions.push(act);
        }

        tracing::info!(id, act_count = act_executions.len(), "Loaded narrative execution");
        Ok(rows_to_narrative_execution(
            &execution_row,
            execution_row.narrative_name.clone(),
            act_executions,
        ))
    }

    #[tracing::instrument(skip(self, filter), fields(
        narrative_name = ?filter.narrative_name(),
        status = ?filter.status(),
        offset = ?filter.offset(),
        limit = ?filter.limit()
    ))]
    async fn list_executions(
        &self,
        filter: &ExecutionFilter,
    ) -> BotticelliResult<Vec<ExecutionSummary>> {
        tracing::debug!("Listing executions with filter");
        let mut conn = self.conn.lock().await;

        let mut query = narrative_executions::table.into_boxed();

        // Apply filters
        if let Some(name) = filter.narrative_name() {
            query = query.filter(narrative_executions::narrative_name.eq(name));
        }

        if let Some(status) = filter.status() {
            query = query.filter(narrative_executions::status.eq(status_to_string(*status)));
        }

        // Note: Date filtering removed - ExecutionFilter in interface doesn't have these fields
        // Original code filtered by started_after and started_before

        // Order by started_at descending (most recent first)
        query = query.order(narrative_executions::started_at.desc());

        // Apply offset and limit
        if let Some(offset) = filter.offset() {
            query = query.offset(*offset as i64);
        }

        if let Some(limit) = filter.limit() {
            query = query.limit(*limit as i64);
        }

        let execution_rows: Vec<NarrativeExecutionRow> = query.load(&mut *conn).map_err(|e| {
            tracing::error!(error = ?e, "Failed to list executions");
            BotticelliError::from(BackendError::new(format!(
                "Failed to list executions: {}",
                e
            )))
        })?;

        tracing::debug!(row_count = execution_rows.len(), "Loaded execution rows");

        // Count acts for each execution
        let mut summaries = Vec::new();
        for row in execution_rows {
            let act_count: i64 = act_executions::table
                .filter(act_executions::execution_id.eq(row.id))
                .count()
                .get_result(&mut *conn)
                .map_err(|e| {
                    tracing::error!(error = ?e, "Failed to count acts");
                    BotticelliError::from(BackendError::new(format!("Failed to count acts: {}", e)))
                })?;

            summaries.push(ExecutionSummary::new(
                row.id,
                row.narrative_name,
                row.narrative_description,
                string_to_status(&row.status)?,
                act_count as usize,
                row.error_message,
            ));
        }

        tracing::info!(summary_count = summaries.len(), "Listed executions");
        Ok(summaries)
    }

    #[tracing::instrument(skip(self), fields(id, status = ?status))]
    async fn update_status(&self, id: i32, status: ExecutionStatus) -> BotticelliResult<()> {
        tracing::debug!("Updating execution status");
        let mut conn = self.conn.lock().await;

        let status_str = status_to_string(status);
        let completed_at = match status {
            ExecutionStatus::Completed | ExecutionStatus::Failed => Some(Utc::now().naive_utc()),
            ExecutionStatus::Running => None,
        };

        diesel::update(narrative_executions::table.find(id))
            .set((
                narrative_executions::status.eq(status_str),
                narrative_executions::completed_at.eq(completed_at),
            ))
            .execute(&mut *conn)
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to update execution status");
                BotticelliError::from(BackendError::new(format!(
                    "Failed to update execution status: {}",
                    e
                )))
            })?;

        tracing::info!(id, "Updated execution status");
        Ok(())
    }

    #[tracing::instrument(skip(self), fields(id))]
    async fn delete_execution(&self, id: i32) -> BotticelliResult<()> {
        tracing::debug!("Deleting execution");
        let mut conn = self.conn.lock().await;

        diesel::delete(narrative_executions::table.find(id))
            .execute(&mut *conn)
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to delete execution");
                BotticelliError::from(BackendError::new(format!(
                    "Failed to delete execution: {}",
                    e
                )))
            })?;

        tracing::info!(id, "Deleted execution");
        Ok(())
    }

    // Media storage methods
    #[tracing::instrument(skip(self, data, metadata), fields(size = data.len(), media_type = %metadata.media_type()))]
    async fn store_media(
        &self,
        data: &[u8],
        metadata: &botticelli_storage::MediaMetadata,
    ) -> BotticelliResult<botticelli_storage::MediaReference> {
        tracing::debug!("Storing media");
        use crate::schema::media_references;
        use sha2::{Digest, Sha256};

        // Compute hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());
        tracing::debug!(hash = %hash, "Computed content hash");

        // Check if already exists
        if let Some(existing) = self.get_media_by_hash(&hash).await? {
            tracing::debug!(
                hash = %hash,
                id = %existing.id(),
                "Media already exists, returning existing reference"
            );
            return Ok(existing);
        }

        // Store in backend
        let reference = self.storage.store(data, metadata).await?;

        // Save reference to database
        let mut conn = self.conn.lock().await;

        #[derive(Insertable)]
        #[diesel(table_name = media_references)]
        struct NewMediaReferenceRow {
            id: uuid::Uuid,
            media_type: String,
            mime_type: String,
            size_bytes: i64,
            content_hash: String,
            storage_backend: String,
            storage_path: String,
            width: Option<i32>,
            height: Option<i32>,
            duration_seconds: Option<f32>,
        }

        let new_row = NewMediaReferenceRow {
            id: *reference.id(),
            media_type: reference.media_type().as_str().to_string(),
            mime_type: reference.mime_type().clone(),
            size_bytes: *reference.size_bytes(),
            content_hash: reference.content_hash().clone(),
            storage_backend: reference.storage_backend().clone(),
            storage_path: reference.storage_path().clone(),
            width: metadata.width().map(|w| w as i32),
            height: metadata.height().map(|h| h as i32),
            duration_seconds: *metadata.duration_seconds(),
        };

        diesel::insert_into(media_references::table)
            .values(&new_row)
            .execute(&mut *conn)
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to save media reference");
                BotticelliError::from(BackendError::new(format!(
                    "Failed to save media reference: {}",
                    e
                )))
            })?;

        tracing::info!(
            id = %reference.id(),
            hash = %reference.content_hash(),
            media_type = %reference.media_type(),
            size_bytes = reference.size_bytes(),
            "Stored media in database"
        );

        Ok(reference)
    }

    #[tracing::instrument(skip(self, reference), fields(id = %reference.id(), hash = %reference.content_hash()))]
    async fn load_media(
        &self,
        reference: &botticelli_storage::MediaReference,
    ) -> BotticelliResult<Vec<u8>> {
        tracing::debug!("Loading media");
        let data = self.storage.retrieve(reference).await?;
        tracing::info!(size = data.len(), "Loaded media");
        Ok(data)
    }

    #[tracing::instrument(skip(self), fields(hash = content_hash))]
    async fn get_media_by_hash(
        &self,
        content_hash: &str,
    ) -> BotticelliResult<Option<botticelli_storage::MediaReference>> {
        tracing::debug!("Looking up media by hash");
        use crate::schema::media_references;

        let mut conn = self.conn.lock().await;

        let result: Option<(uuid::Uuid, String, String, i64, String, String, String)> =
            media_references::table
                .select((
                    media_references::id,
                    media_references::media_type,
                    media_references::mime_type,
                    media_references::size_bytes,
                    media_references::content_hash,
                    media_references::storage_backend,
                    media_references::storage_path,
                ))
                .filter(media_references::content_hash.eq(content_hash))
                .first(&mut *conn)
                .optional()
                .map_err(|e| {
                    tracing::error!(error = ?e, "Failed to query media by hash");
                    BotticelliError::from(BackendError::new(format!(
                        "Failed to query media by hash: {}",
                        e
                    )))
                })?;

        let found = result.is_some();
        tracing::debug!(found, "Media lookup complete");
        Ok(result.map(
            |(id, media_type_str, mime_type, size_bytes, hash, backend, path)| {
                botticelli_storage::MediaReferenceBuilder::default()
                    .id(id)
                    .media_type(
                        media_type_str
                            .parse()
                            .unwrap_or(botticelli_storage::MediaType::Image),
                    )
                    .mime_type(mime_type)
                    .size_bytes(size_bytes)
                    .content_hash(hash)
                    .storage_backend(backend)
                    .storage_path(path)
                    .build()
                    .expect("Valid MediaReference")
            },
        ))
    }

    // Video methods use default implementations from trait (return NotImplemented)
}

// Implement NarrativeRegistryOperations trait for MCP tool integration
#[async_trait]
impl botticelli_interface::NarrativeRegistryOperations for PostgresNarrativeRepository {
    type Error = BotticelliError;
    type Execution = NarrativeExecution;
    type Status = ExecutionStatus;
    type Filter = ExecutionFilter;
    type Summary = ExecutionSummary;

    async fn save_execution(&self, execution: &Self::Execution) -> Result<i32, Self::Error> {
        NarrativeRepository::save_execution(self, execution).await
    }

    async fn load_execution(&self, id: i32) -> BotticelliResult<NarrativeExecution> {
        NarrativeRepository::load_execution(self, id).await
    }

    async fn update_status(&self, id: i32, status: ExecutionStatus) -> BotticelliResult<()> {
        NarrativeRepository::update_status(self, id, status).await
    }

    async fn list_executions(
        &self,
        filter: &ExecutionFilter,
    ) -> BotticelliResult<Vec<ExecutionSummary>> {
        NarrativeRepository::list_executions(self, filter).await
    }

    async fn delete_execution(&self, id: i32) -> BotticelliResult<()> {
        NarrativeRepository::delete_execution(self, id).await
    }
}

#[async_trait]
impl NarrativeStorageOperations for PostgresNarrativeRepository {
    type Error = BotticelliError;

    #[tracing::instrument(skip(self), fields(pattern = ?_pattern))]
    async fn list_narratives(&self, _pattern: Option<&str>) -> BotticelliResult<Vec<String>> {
        tracing::debug!("Listing narratives from database");
        // For database implementation, we list narrative IDs from the database
        let mut conn = self.conn.lock().await;
        let results: Vec<i32> = narrative_executions::table
            .select(narrative_executions::id)
            .load(&mut *conn as &mut PgConnection)
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to list narrative IDs");
                BotticelliError::from(BackendError::new(format!(
                    "Failed to list narrative IDs: {}",
                    e
                )))
            })?;

        tracing::info!(count = results.len(), "Listed narratives");
        Ok(results.iter().map(|id| id.to_string()).collect())
    }

    #[tracing::instrument(skip(self), fields(filename))]
    async fn load_narrative(&self, filename: &str) -> BotticelliResult<Value> {
        tracing::debug!("Loading narrative by filename");
        // Parse filename as narrative ID
        let id: i32 = filename.parse().map_err(|e| {
            tracing::error!(error = ?e, "Invalid narrative ID");
            BotticelliError::from(BackendError::new(format!(
                "Invalid narrative ID '{}': {}",
                filename, e
            )))
        })?;

        let execution = self.load_execution(id).await?;
        let value = serde_json::to_value(&execution).map_err(|e| {
            tracing::error!(error = ?e, "Failed to serialize narrative");
            BotticelliError::from(BackendError::new(format!(
                "Failed to serialize narrative: {}",
                e
            )))
        })?;

        tracing::info!(id, "Loaded narrative");
        Ok(value)
    }

    #[tracing::instrument(skip(self, toml_content), fields(content_len = toml_content.len()))]
    async fn validate_narrative(&self, toml_content: &str) -> BotticelliResult<Value> {
        tracing::debug!("Validating narrative TOML");
        // For database implementation, validate TOML can be parsed
        let value = toml::from_str::<Value>(toml_content)
            .map_err(|e| {
                tracing::error!(error = ?e, "Invalid TOML");
                BotticelliError::from(BackendError::new(format!("Invalid TOML: {}", e)))
            })?;

        tracing::info!("Validated narrative TOML");
        Ok(value)
    }

    #[tracing::instrument(skip(self, toml_content), fields(content_len = toml_content.len(), name_override = ?_name_override))]
    async fn parse_narrative(
        &self,
        toml_content: &str,
        _name_override: Option<&str>,
    ) -> BotticelliResult<Value> {
        tracing::debug!("Parsing narrative TOML");
        // Parse TOML content
        let parsed: Value = toml::from_str(toml_content).map_err(|e| {
            tracing::error!(error = ?e, "Failed to parse TOML");
            BotticelliError::from(BackendError::new(format!("Failed to parse TOML: {}", e)))
        })?;

        tracing::info!("Parsed narrative TOML");
        Ok(parsed)
    }
}
