//! [`RedbStorage`] — `BotStorage` implementation backed by an embedded redb database.
//!
//! Uses [`elicit_redb::RedbBackend`] for all persistence. Each logical table
//! becomes a named redb table; rows are stored as `String → JSON` entries.

use elicit_db::{DbError, DbErrorKind, DbKvStore, DbValue};
use elicit_redb::RedbBackend;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use botticelli_interface::{
    ActExecutionRecord, ActInputRecord, ActorServerExecutionRecord, ActorServerStateRecord,
    ActorStateStore, BotStorage, BotStorageResult, ContentGenerationRecord, ContentRecord,
    ContentStore, ModelResponseRecord, NarrativeExecutionRecord, NarrativeStore, PostHistoryRecord,
    PostStore,
};

// ── Table name constants ───────────────────────────────────────────────────────

const NARRATIVE_EXECUTIONS: &str = "narrative_executions";
const ACT_EXECUTIONS: &str = "act_executions";
const ACT_INPUTS: &str = "act_inputs";
const ACTOR_SERVER_STATE: &str = "actor_server_state";
const ACTOR_SERVER_EXECUTIONS: &str = "actor_server_executions";
const CONTENT: &str = "content";
const CONTENT_GENERATIONS: &str = "content_generations";
const MODEL_RESPONSES: &str = "model_responses";
const POST_HISTORY: &str = "post_history";

// ── RedbStorage ───────────────────────────────────────────────────────────────

/// A `BotStorage` backend backed by an embedded redb file.
///
/// Obtain via [`RedbStorage::open`] for a persistent file or
/// [`RedbStorage::in_memory`] for tests.
pub struct RedbStorage(RedbBackend);

impl RedbStorage {
    /// Open or create a redb database at `path`.
    #[instrument(skip_all, fields(path = %path.display()))]
    pub fn open(path: &std::path::Path) -> BotStorageResult<Self> {
        let path_str = path.to_string_lossy();
        Ok(Self(RedbBackend::open(&path_str)?))
    }

    /// Create a transient in-memory redb instance (for tests).
    #[instrument]
    pub fn in_memory() -> BotStorageResult<Self> {
        Ok(Self(RedbBackend::in_memory()?))
    }

    /// Serialize `value` and write it to `table` under `key`.
    #[instrument(skip(self, value), fields(%table, %key))]
    async fn put<T: Serialize>(&self, table: &str, key: &str, value: &T) -> BotStorageResult<()> {
        let json = serde_json::to_value(value)?;
        self.0
            .kv_insert(table, DbValue::Text(key.to_owned()), DbValue::Json(json))
            .await?;
        Ok(())
    }

    /// Read a single record from `table` by `key`.
    #[instrument(skip(self), fields(%table, %key))]
    async fn get_one<T: for<'de> Deserialize<'de>>(
        &self,
        table: &str,
        key: &str,
    ) -> BotStorageResult<Option<T>> {
        match self.0.kv_get(table, &DbValue::Text(key.to_owned())).await? {
            Some(DbValue::Json(v)) => Ok(Some(serde_json::from_value(v)?)),
            Some(_) => Err(DbError::new(DbErrorKind::Serialization(
                "expected JSON value in redb store".to_string(),
            ))
            .into()),
            None => Ok(None),
        }
    }

    /// Scan all entries in `table` and deserialize each value.
    #[instrument(skip(self), fields(%table))]
    async fn scan_all<T: for<'de> Deserialize<'de>>(
        &self,
        table: &str,
    ) -> BotStorageResult<Vec<T>> {
        let entries = self.0.kv_scan(table).await?;
        entries
            .into_iter()
            .map(|e| match e.value {
                DbValue::Json(v) => Ok(serde_json::from_value(v)?),
                _ => Err(DbError::new(DbErrorKind::Serialization(
                    "expected JSON value in redb store".to_string(),
                ))
                .into()),
            })
            .collect()
    }

    /// Remove a single record from `table` by `key`.
    #[instrument(skip(self), fields(%table, %key))]
    async fn remove(&self, table: &str, key: &str) -> BotStorageResult<()> {
        self.0
            .kv_remove(table, &DbValue::Text(key.to_owned()))
            .await?;
        Ok(())
    }
}

// ── NarrativeStore ────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl NarrativeStore for RedbStorage {
    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_narrative_execution(
        &self,
        record: &NarrativeExecutionRecord,
    ) -> BotStorageResult<()> {
        self.put(NARRATIVE_EXECUTIONS, &record.id, record).await
    }

    #[instrument(skip(self), fields(%id))]
    async fn get_narrative_execution(
        &self,
        id: &str,
    ) -> BotStorageResult<Option<NarrativeExecutionRecord>> {
        self.get_one(NARRATIVE_EXECUTIONS, id).await
    }

    #[instrument(skip(self), fields(%limit))]
    async fn list_narrative_executions(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<NarrativeExecutionRecord>> {
        let mut records: Vec<NarrativeExecutionRecord> =
            self.scan_all(NARRATIVE_EXECUTIONS).await?;
        records.sort_by_key(|r| std::cmp::Reverse(r.started_at));
        records.truncate(limit);
        Ok(records)
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_act_execution(&self, record: &ActExecutionRecord) -> BotStorageResult<()> {
        self.put(ACT_EXECUTIONS, &record.id, record).await
    }

    #[instrument(skip(self), fields(%narrative_execution_id))]
    async fn list_act_executions(
        &self,
        narrative_execution_id: &str,
    ) -> BotStorageResult<Vec<ActExecutionRecord>> {
        let all: Vec<ActExecutionRecord> = self.scan_all(ACT_EXECUTIONS).await?;
        let mut matching: Vec<_> = all
            .into_iter()
            .filter(|r| r.narrative_execution_id == narrative_execution_id)
            .collect();
        matching.sort_by_key(|r| r.sequence_number);
        Ok(matching)
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_act_input(&self, record: &ActInputRecord) -> BotStorageResult<()> {
        self.put(ACT_INPUTS, &record.id, record).await
    }
}

// ── ActorStateStore ───────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl ActorStateStore for RedbStorage {
    #[instrument(skip(self, record), fields(task_id = %record.task_id))]
    async fn save_actor_state(&self, record: &ActorServerStateRecord) -> BotStorageResult<()> {
        self.put(ACTOR_SERVER_STATE, &record.task_id, record).await
    }

    #[instrument(skip(self), fields(%task_id))]
    async fn get_actor_state(
        &self,
        task_id: &str,
    ) -> BotStorageResult<Option<ActorServerStateRecord>> {
        self.get_one(ACTOR_SERVER_STATE, task_id).await
    }

    #[instrument(skip(self))]
    async fn list_actor_states(&self) -> BotStorageResult<Vec<ActorServerStateRecord>> {
        self.scan_all(ACTOR_SERVER_STATE).await
    }

    #[instrument(skip(self), fields(%task_id))]
    async fn delete_actor_state(&self, task_id: &str) -> BotStorageResult<()> {
        self.remove(ACTOR_SERVER_STATE, task_id).await
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_actor_execution(
        &self,
        record: &ActorServerExecutionRecord,
    ) -> BotStorageResult<()> {
        self.put(ACTOR_SERVER_EXECUTIONS, &record.id, record).await
    }

    #[instrument(skip(self), fields(%task_id, %limit))]
    async fn list_actor_executions(
        &self,
        task_id: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<ActorServerExecutionRecord>> {
        let all: Vec<ActorServerExecutionRecord> = self.scan_all(ACTOR_SERVER_EXECUTIONS).await?;
        let mut matching: Vec<_> = all.into_iter().filter(|r| r.task_id == task_id).collect();
        matching.sort_by_key(|r| std::cmp::Reverse(r.started_at));
        matching.truncate(limit);
        Ok(matching)
    }

    #[instrument(skip(self), fields(%limit))]
    async fn list_all_actor_executions(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<ActorServerExecutionRecord>> {
        let mut records: Vec<ActorServerExecutionRecord> =
            self.scan_all(ACTOR_SERVER_EXECUTIONS).await?;
        records.sort_by_key(|r| std::cmp::Reverse(r.started_at));
        records.truncate(limit);
        Ok(records)
    }

    #[instrument(skip(self), fields(%id))]
    async fn delete_actor_execution(&self, id: &str) -> BotStorageResult<()> {
        self.remove(ACTOR_SERVER_EXECUTIONS, id).await
    }
}

// ── ContentStore ──────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl ContentStore for RedbStorage {
    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_content(&self, record: &ContentRecord) -> BotStorageResult<()> {
        self.put(CONTENT, &record.id, record).await
    }

    #[instrument(skip(self), fields(%id))]
    async fn get_content(&self, id: &str) -> BotStorageResult<Option<ContentRecord>> {
        self.get_one(CONTENT, id).await
    }

    #[instrument(skip(self), fields(%table_name, %limit))]
    async fn list_content(
        &self,
        table_name: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<ContentRecord>> {
        let all: Vec<ContentRecord> = self.scan_all(CONTENT).await?;
        let mut matching: Vec<_> = all
            .into_iter()
            .filter(|r| r.table_name == table_name)
            .collect();
        matching.sort_by_key(|r| std::cmp::Reverse(r.created_at));
        matching.truncate(limit);
        Ok(matching)
    }

    #[instrument(skip(self), fields(%id))]
    async fn delete_content(&self, id: &str) -> BotStorageResult<()> {
        self.remove(CONTENT, id).await
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_content_generation(
        &self,
        record: &ContentGenerationRecord,
    ) -> BotStorageResult<()> {
        self.put(CONTENT_GENERATIONS, &record.id, record).await
    }

    #[instrument(skip(self), fields(%table_name, %limit))]
    async fn list_content_generations_by_table(
        &self,
        table_name: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<ContentGenerationRecord>> {
        let all: Vec<ContentGenerationRecord> = self.scan_all(CONTENT_GENERATIONS).await?;
        let mut matching: Vec<_> = all
            .into_iter()
            .filter(|r| r.table_name == table_name)
            .collect();
        matching.sort_by_key(|r| std::cmp::Reverse(r.generated_at));
        matching.truncate(limit);
        Ok(matching)
    }

    #[instrument(skip(self), fields(%limit))]
    async fn list_content_generations(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<ContentGenerationRecord>> {
        let mut records: Vec<ContentGenerationRecord> = self.scan_all(CONTENT_GENERATIONS).await?;
        records.sort_by_key(|r| std::cmp::Reverse(r.generated_at));
        records.truncate(limit);
        Ok(records)
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_model_response(&self, record: &ModelResponseRecord) -> BotStorageResult<()> {
        self.put(MODEL_RESPONSES, &record.id, record).await
    }

    #[instrument(skip(self), fields(%limit))]
    async fn list_model_responses(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<ModelResponseRecord>> {
        let mut records: Vec<ModelResponseRecord> = self.scan_all(MODEL_RESPONSES).await?;
        records.sort_by_key(|r| std::cmp::Reverse(r.created_at));
        records.truncate(limit);
        Ok(records)
    }
}

// ── PostStore ─────────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl PostStore for RedbStorage {
    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_post_history(&self, record: &PostHistoryRecord) -> BotStorageResult<()> {
        self.put(POST_HISTORY, &record.id, record).await
    }

    #[instrument(skip(self), fields(%platform, %limit))]
    async fn list_post_history(
        &self,
        platform: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<PostHistoryRecord>> {
        let all: Vec<PostHistoryRecord> = self.scan_all(POST_HISTORY).await?;
        let mut matching: Vec<_> = all.into_iter().filter(|r| r.platform == platform).collect();
        matching.sort_by_key(|r| std::cmp::Reverse(r.posted_at));
        matching.truncate(limit);
        Ok(matching)
    }
}

// ── BotStorage ────────────────────────────────────────────────────────────────

impl BotStorage for RedbStorage {}
