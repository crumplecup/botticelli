//! [`PostgresStorage`] — `BotStorage` implementation backed by PostgreSQL via elicit_sqlx.
//!
//! Uses [`elicit_sqlx::SqlxDbBackend`] and the [`elicit_db::DbQueryExecutor`] trait
//! for all queries. Tables are created on first connect; all columns use types
//! compatible with the sqlx `AnyPool` driver (TEXT, INTEGER, BOOLEAN, REAL).

use chrono::{DateTime, Utc};
use elicit_db::{DbQueryExecutor, DbRow, DbValue};
use elicit_sqlx::SqlxDbBackend;
use tracing::instrument;

use botticelli_interface::{
    ActExecutionRecord, ActInputRecord, ActorServerExecutionRecord, ActorServerStateRecord,
    ActorStateStore, BotStorage, BotStorageError, BotStorageResult, ContentGenerationRecord,
    ContentRecord, ContentStore, ModelResponseRecord, NarrativeExecutionRecord, NarrativeStore,
    PostHistoryRecord, PostStore,
};

// ── Connect error ─────────────────────────────────────────────────────────────

/// Error returned by [`PostgresStorage::connect`].
///
/// Separate from [`BotStorageError`] because the constructor surfaces a raw
/// `sqlx::Error` before the [`elicit_db`] abstraction layer is in play.
#[derive(Debug, derive_more::Display, derive_more::Error, derive_more::From)]
pub enum PostgresConnectError {
    /// sqlx driver-level connection failure.
    #[display("sqlx connection error: {}", _0)]
    Sqlx(sqlx::Error),
    /// Schema creation failed after connecting.
    #[display("schema init error: {}", _0)]
    SchemaInit(BotStorageError),
}

// ── PostgresStorage ───────────────────────────────────────────────────────────

/// A `BotStorage` backend backed by PostgreSQL.
///
/// Obtain via [`PostgresStorage::connect`].
pub struct PostgresStorage(SqlxDbBackend);

impl PostgresStorage {
    /// Connect to a Postgres URL and ensure all tables exist.
    #[instrument(skip_all, fields(url))]
    pub async fn connect(database_url: &str) -> Result<Self, PostgresConnectError> {
        let backend = SqlxDbBackend::connect(database_url).await?;
        let storage = Self(backend);
        storage
            .ensure_schema()
            .await
            .map_err(PostgresConnectError::SchemaInit)?;
        Ok(storage)
    }

    /// Create all tables if they do not yet exist.
    #[instrument(skip(self))]
    async fn ensure_schema(&self) -> BotStorageResult<()> {
        let ddl = [
            "CREATE TABLE IF NOT EXISTS narrative_executions (
                id TEXT PRIMARY KEY,
                narrative_name TEXT NOT NULL,
                narrative_description TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                status TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS act_executions (
                id TEXT PRIMARY KEY,
                narrative_execution_id TEXT NOT NULL,
                act_name TEXT NOT NULL,
                sequence_number INTEGER NOT NULL,
                model TEXT,
                temperature REAL,
                max_tokens INTEGER,
                response TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS act_inputs (
                id TEXT PRIMARY KEY,
                act_execution_id TEXT NOT NULL,
                input_order INTEGER NOT NULL,
                input_type TEXT NOT NULL,
                text_content TEXT,
                mime_type TEXT,
                filename TEXT,
                media_ref_id TEXT,
                created_at TEXT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS actor_server_state (
                task_id TEXT PRIMARY KEY,
                actor_name TEXT NOT NULL,
                last_run TEXT,
                next_run TEXT NOT NULL,
                consecutive_failures INTEGER NOT NULL,
                is_paused BOOLEAN NOT NULL,
                metadata TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS actor_server_executions (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                actor_name TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                success BOOLEAN NOT NULL,
                error_message TEXT,
                skills_succeeded INTEGER NOT NULL,
                skills_failed INTEGER NOT NULL,
                skills_skipped INTEGER NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS content (
                id TEXT PRIMARY KEY,
                table_name TEXT NOT NULL,
                content_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS content_generations (
                id TEXT PRIMARY KEY,
                table_name TEXT NOT NULL,
                narrative_file TEXT NOT NULL,
                narrative_name TEXT NOT NULL,
                generated_at TEXT NOT NULL,
                completed_at TEXT,
                row_count INTEGER,
                generation_duration_ms INTEGER,
                status TEXT NOT NULL,
                error_message TEXT,
                created_by TEXT
            )",
            "CREATE TABLE IF NOT EXISTS model_responses (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                provider TEXT NOT NULL,
                model_name TEXT NOT NULL,
                request_messages TEXT NOT NULL,
                request_temperature REAL,
                request_max_tokens INTEGER,
                request_model TEXT,
                response_outputs TEXT NOT NULL,
                duration_ms INTEGER,
                error_message TEXT
            )",
            "CREATE TABLE IF NOT EXISTS post_history (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                post_id TEXT NOT NULL,
                content_json TEXT NOT NULL,
                posted_at TEXT NOT NULL
            )",
        ];
        for stmt in &ddl {
            self.0.execute(stmt, &[]).await?;
        }
        Ok(())
    }
}

// ── Row extraction helpers ────────────────────────────────────────────────────

fn col_text(row: &DbRow, name: &str) -> BotStorageResult<String> {
    match row.get(name) {
        Some(DbValue::Text(s)) => Ok(s.clone()),
        _ => Err(BotStorageError::MissingColumn(name.to_string())),
    }
}

fn col_opt_text(row: &DbRow, name: &str) -> Option<String> {
    match row.get(name) {
        Some(DbValue::Text(s)) => Some(s.clone()),
        _ => None,
    }
}

fn col_i32(row: &DbRow, name: &str) -> BotStorageResult<i32> {
    match row.get(name) {
        Some(DbValue::Int(n)) => Ok(*n as i32),
        _ => Err(BotStorageError::MissingColumn(name.to_string())),
    }
}

fn col_opt_i32(row: &DbRow, name: &str) -> Option<i32> {
    match row.get(name)? {
        DbValue::Int(n) => Some(*n as i32),
        _ => None,
    }
}

fn col_bool(row: &DbRow, name: &str) -> BotStorageResult<bool> {
    match row.get(name) {
        Some(DbValue::Bool(b)) => Ok(*b),
        _ => Err(BotStorageError::MissingColumn(name.to_string())),
    }
}

fn col_opt_f32(row: &DbRow, name: &str) -> Option<f32> {
    match row.get(name)? {
        DbValue::Float(f) => Some(*f as f32),
        _ => None,
    }
}

fn col_dt(row: &DbRow, name: &str) -> BotStorageResult<DateTime<Utc>> {
    Ok(col_text(row, name)?.parse::<DateTime<Utc>>()?)
}

fn col_opt_dt(row: &DbRow, name: &str) -> BotStorageResult<Option<DateTime<Utc>>> {
    match col_opt_text(row, name) {
        Some(s) => Ok(Some(s.parse::<DateTime<Utc>>()?)),
        None => Ok(None),
    }
}

fn col_json(row: &DbRow, name: &str) -> BotStorageResult<serde_json::Value> {
    Ok(serde_json::from_str(&col_text(row, name)?)?)
}

fn json_str(v: &serde_json::Value) -> BotStorageResult<String> {
    Ok(serde_json::to_string(v)?)
}

// ── NarrativeStore ────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl NarrativeStore for PostgresStorage {
    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_narrative_execution(
        &self,
        record: &NarrativeExecutionRecord,
    ) -> BotStorageResult<()> {
        let sql = "INSERT INTO narrative_executions \
            (id, narrative_name, narrative_description, started_at, completed_at, \
             status, error_message, created_at) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
            ON CONFLICT (id) DO UPDATE SET \
                narrative_name = EXCLUDED.narrative_name, \
                narrative_description = EXCLUDED.narrative_description, \
                started_at = EXCLUDED.started_at, \
                completed_at = EXCLUDED.completed_at, \
                status = EXCLUDED.status, \
                error_message = EXCLUDED.error_message";
        let params = [
            DbValue::Text(record.id.clone()),
            DbValue::Text(record.narrative_name.clone()),
            opt_text(&record.narrative_description),
            DbValue::Text(record.started_at.to_rfc3339()),
            opt_text(&record.completed_at.map(|d| d.to_rfc3339())),
            DbValue::Text(record.status.clone()),
            opt_text(&record.error_message),
            DbValue::Text(record.created_at.to_rfc3339()),
        ];
        self.0.execute(sql, &params).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(%id))]
    async fn get_narrative_execution(
        &self,
        id: &str,
    ) -> BotStorageResult<Option<NarrativeExecutionRecord>> {
        let sql = "SELECT id, narrative_name, narrative_description, started_at, completed_at, \
                   status, error_message, created_at \
                   FROM narrative_executions WHERE id = $1";
        let (rows, _) = self
            .0
            .query_rows(sql, &[DbValue::Text(id.to_owned())])
            .await?;
        match rows.rows.first() {
            Some(row) => decode_narrative_execution(row).map(Some),
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(%limit))]
    async fn list_narrative_executions(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<NarrativeExecutionRecord>> {
        let sql = "SELECT id, narrative_name, narrative_description, started_at, completed_at, \
                   status, error_message, created_at \
                   FROM narrative_executions ORDER BY started_at DESC LIMIT $1";
        let (rows, _) = self
            .0
            .query_rows(sql, &[DbValue::Int(limit as i64)])
            .await?;
        rows.rows.iter().map(decode_narrative_execution).collect()
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_act_execution(&self, record: &ActExecutionRecord) -> BotStorageResult<()> {
        let sql = "INSERT INTO act_executions \
            (id, narrative_execution_id, act_name, sequence_number, model, temperature, \
             max_tokens, response, created_at) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
            ON CONFLICT (id) DO UPDATE SET \
                narrative_execution_id = EXCLUDED.narrative_execution_id, \
                act_name = EXCLUDED.act_name, \
                sequence_number = EXCLUDED.sequence_number, \
                model = EXCLUDED.model, \
                temperature = EXCLUDED.temperature, \
                max_tokens = EXCLUDED.max_tokens, \
                response = EXCLUDED.response";
        let params = [
            DbValue::Text(record.id.clone()),
            DbValue::Text(record.narrative_execution_id.clone()),
            DbValue::Text(record.act_name.clone()),
            DbValue::Int(record.sequence_number as i64),
            opt_text(&record.model),
            opt_float(record.temperature.map(|f| f as f64)),
            opt_int(record.max_tokens.map(|i| i as i64)),
            DbValue::Text(record.response.clone()),
            DbValue::Text(record.created_at.to_rfc3339()),
        ];
        self.0.execute(sql, &params).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(%narrative_execution_id))]
    async fn list_act_executions(
        &self,
        narrative_execution_id: &str,
    ) -> BotStorageResult<Vec<ActExecutionRecord>> {
        let sql = "SELECT id, narrative_execution_id, act_name, sequence_number, model, \
                   temperature, max_tokens, response, created_at \
                   FROM act_executions WHERE narrative_execution_id = $1 \
                   ORDER BY sequence_number";
        let (rows, _) = self
            .0
            .query_rows(sql, &[DbValue::Text(narrative_execution_id.to_owned())])
            .await?;
        rows.rows.iter().map(decode_act_execution).collect()
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_act_input(&self, record: &ActInputRecord) -> BotStorageResult<()> {
        let sql = "INSERT INTO act_inputs \
            (id, act_execution_id, input_order, input_type, text_content, mime_type, \
             filename, media_ref_id, created_at) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
            ON CONFLICT (id) DO NOTHING";
        let params = [
            DbValue::Text(record.id.clone()),
            DbValue::Text(record.act_execution_id.clone()),
            DbValue::Int(record.input_order as i64),
            DbValue::Text(record.input_type.clone()),
            opt_text(&record.text_content),
            opt_text(&record.mime_type),
            opt_text(&record.filename),
            opt_text(&record.media_ref_id),
            DbValue::Text(record.created_at.to_rfc3339()),
        ];
        self.0.execute(sql, &params).await?;
        Ok(())
    }
}

// ── ActorStateStore ───────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl ActorStateStore for PostgresStorage {
    #[instrument(skip(self, record), fields(task_id = %record.task_id))]
    async fn save_actor_state(&self, record: &ActorServerStateRecord) -> BotStorageResult<()> {
        let metadata = json_str(&record.metadata)?;
        let sql = "INSERT INTO actor_server_state \
            (task_id, actor_name, last_run, next_run, consecutive_failures, is_paused, \
             metadata, updated_at) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
            ON CONFLICT (task_id) DO UPDATE SET \
                actor_name = EXCLUDED.actor_name, \
                last_run = EXCLUDED.last_run, \
                next_run = EXCLUDED.next_run, \
                consecutive_failures = EXCLUDED.consecutive_failures, \
                is_paused = EXCLUDED.is_paused, \
                metadata = EXCLUDED.metadata, \
                updated_at = EXCLUDED.updated_at";
        let params = [
            DbValue::Text(record.task_id.clone()),
            DbValue::Text(record.actor_name.clone()),
            opt_text(&record.last_run.map(|d| d.to_rfc3339())),
            DbValue::Text(record.next_run.to_rfc3339()),
            DbValue::Int(record.consecutive_failures as i64),
            DbValue::Bool(record.is_paused),
            DbValue::Text(metadata),
            DbValue::Text(record.updated_at.to_rfc3339()),
        ];
        self.0.execute(sql, &params).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(%task_id))]
    async fn get_actor_state(
        &self,
        task_id: &str,
    ) -> BotStorageResult<Option<ActorServerStateRecord>> {
        let sql = "SELECT task_id, actor_name, last_run, next_run, consecutive_failures, \
                   is_paused, metadata, updated_at \
                   FROM actor_server_state WHERE task_id = $1";
        let (rows, _) = self
            .0
            .query_rows(sql, &[DbValue::Text(task_id.to_owned())])
            .await?;
        match rows.rows.first() {
            Some(row) => decode_actor_state(row).map(Some),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn list_actor_states(&self) -> BotStorageResult<Vec<ActorServerStateRecord>> {
        let sql = "SELECT task_id, actor_name, last_run, next_run, consecutive_failures, \
                   is_paused, metadata, updated_at FROM actor_server_state ORDER BY actor_name";
        let (rows, _) = self.0.query_rows(sql, &[]).await?;
        rows.rows.iter().map(decode_actor_state).collect()
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_actor_execution(
        &self,
        record: &ActorServerExecutionRecord,
    ) -> BotStorageResult<()> {
        let metadata = json_str(&record.metadata)?;
        let sql = "INSERT INTO actor_server_executions \
            (id, task_id, actor_name, started_at, completed_at, success, error_message, \
             skills_succeeded, skills_failed, skills_skipped, metadata, created_at) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
            ON CONFLICT (id) DO UPDATE SET \
                completed_at = EXCLUDED.completed_at, \
                success = EXCLUDED.success, \
                error_message = EXCLUDED.error_message, \
                skills_succeeded = EXCLUDED.skills_succeeded, \
                skills_failed = EXCLUDED.skills_failed, \
                skills_skipped = EXCLUDED.skills_skipped, \
                metadata = EXCLUDED.metadata";
        let params = [
            DbValue::Text(record.id.clone()),
            DbValue::Text(record.task_id.clone()),
            DbValue::Text(record.actor_name.clone()),
            DbValue::Text(record.started_at.to_rfc3339()),
            opt_text(&record.completed_at.map(|d| d.to_rfc3339())),
            DbValue::Bool(record.success),
            opt_text(&record.error_message),
            DbValue::Int(record.skills_succeeded as i64),
            DbValue::Int(record.skills_failed as i64),
            DbValue::Int(record.skills_skipped as i64),
            DbValue::Text(metadata),
            DbValue::Text(record.created_at.to_rfc3339()),
        ];
        self.0.execute(sql, &params).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(%task_id, %limit))]
    async fn list_actor_executions(
        &self,
        task_id: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<ActorServerExecutionRecord>> {
        let sql = "SELECT id, task_id, actor_name, started_at, completed_at, success, \
                   error_message, skills_succeeded, skills_failed, skills_skipped, metadata, \
                   created_at FROM actor_server_executions \
                   WHERE task_id = $1 ORDER BY started_at DESC LIMIT $2";
        let (rows, _) = self
            .0
            .query_rows(
                sql,
                &[
                    DbValue::Text(task_id.to_owned()),
                    DbValue::Int(limit as i64),
                ],
            )
            .await?;
        rows.rows.iter().map(decode_actor_execution).collect()
    }

    #[instrument(skip(self), fields(%limit))]
    async fn list_all_actor_executions(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<ActorServerExecutionRecord>> {
        let sql = "SELECT id, task_id, actor_name, started_at, completed_at, success, \
                   error_message, skills_succeeded, skills_failed, skills_skipped, metadata, \
                   created_at FROM actor_server_executions \
                   ORDER BY started_at DESC LIMIT $1";
        let (rows, _) = self
            .0
            .query_rows(sql, &[DbValue::Int(limit as i64)])
            .await?;
        rows.rows.iter().map(decode_actor_execution).collect()
    }

    #[instrument(skip(self), fields(%task_id))]
    async fn delete_actor_state(&self, task_id: &str) -> BotStorageResult<()> {
        self.0
            .execute(
                "DELETE FROM actor_server_state WHERE task_id = $1",
                &[DbValue::Text(task_id.to_owned())],
            )
            .await?;
        Ok(())
    }

    #[instrument(skip(self), fields(%id))]
    async fn delete_actor_execution(&self, id: &str) -> BotStorageResult<()> {
        self.0
            .execute(
                "DELETE FROM actor_server_executions WHERE id = $1",
                &[DbValue::Text(id.to_owned())],
            )
            .await?;
        Ok(())
    }
}

// ── ContentStore ──────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl ContentStore for PostgresStorage {
    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_content(&self, record: &ContentRecord) -> BotStorageResult<()> {
        let content_json = json_str(&record.content_json)?;
        let sql = "INSERT INTO content (id, table_name, content_json, created_at) \
                   VALUES ($1, $2, $3, $4) \
                   ON CONFLICT (id) DO UPDATE SET \
                       table_name = EXCLUDED.table_name, \
                       content_json = EXCLUDED.content_json";
        let params = [
            DbValue::Text(record.id.clone()),
            DbValue::Text(record.table_name.clone()),
            DbValue::Text(content_json),
            DbValue::Text(record.created_at.to_rfc3339()),
        ];
        self.0.execute(sql, &params).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(%id))]
    async fn get_content(&self, id: &str) -> BotStorageResult<Option<ContentRecord>> {
        let sql = "SELECT id, table_name, content_json, created_at \
                   FROM content WHERE id = $1";
        let (rows, _) = self
            .0
            .query_rows(sql, &[DbValue::Text(id.to_owned())])
            .await?;
        match rows.rows.first() {
            Some(row) => decode_content(row).map(Some),
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(%table_name, %limit))]
    async fn list_content(
        &self,
        table_name: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<ContentRecord>> {
        let sql = "SELECT id, table_name, content_json, created_at \
                   FROM content WHERE table_name = $1 \
                   ORDER BY created_at DESC LIMIT $2";
        let (rows, _) = self
            .0
            .query_rows(
                sql,
                &[
                    DbValue::Text(table_name.to_owned()),
                    DbValue::Int(limit as i64),
                ],
            )
            .await?;
        rows.rows.iter().map(decode_content).collect()
    }

    #[instrument(skip(self), fields(%id))]
    async fn delete_content(&self, id: &str) -> BotStorageResult<()> {
        self.0
            .execute(
                "DELETE FROM content WHERE id = $1",
                &[DbValue::Text(id.to_owned())],
            )
            .await?;
        Ok(())
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_content_generation(
        &self,
        record: &ContentGenerationRecord,
    ) -> BotStorageResult<()> {
        let sql = "INSERT INTO content_generations \
            (id, table_name, narrative_file, narrative_name, generated_at, completed_at, \
             row_count, generation_duration_ms, status, error_message, created_by) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
            ON CONFLICT (id) DO UPDATE SET \
                completed_at = EXCLUDED.completed_at, \
                row_count = EXCLUDED.row_count, \
                generation_duration_ms = EXCLUDED.generation_duration_ms, \
                status = EXCLUDED.status, \
                error_message = EXCLUDED.error_message";
        let params = [
            DbValue::Text(record.id.clone()),
            DbValue::Text(record.table_name.clone()),
            DbValue::Text(record.narrative_file.clone()),
            DbValue::Text(record.narrative_name.clone()),
            DbValue::Text(record.generated_at.to_rfc3339()),
            opt_text(&record.completed_at.map(|d| d.to_rfc3339())),
            opt_int(record.row_count.map(|i| i as i64)),
            opt_int(record.generation_duration_ms.map(|i| i as i64)),
            DbValue::Text(record.status.clone()),
            opt_text(&record.error_message),
            opt_text(&record.created_by),
        ];
        self.0.execute(sql, &params).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(%table_name, %limit))]
    async fn list_content_generations_by_table(
        &self,
        table_name: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<ContentGenerationRecord>> {
        let sql = "SELECT id, table_name, narrative_file, narrative_name, generated_at, \
                   completed_at, row_count, generation_duration_ms, status, error_message, \
                   created_by FROM content_generations \
                   WHERE table_name = $1 ORDER BY generated_at DESC LIMIT $2";
        let (rows, _) = self
            .0
            .query_rows(
                sql,
                &[
                    DbValue::Text(table_name.to_owned()),
                    DbValue::Int(limit as i64),
                ],
            )
            .await?;
        rows.rows.iter().map(decode_content_generation).collect()
    }

    #[instrument(skip(self), fields(%limit))]
    async fn list_content_generations(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<ContentGenerationRecord>> {
        let sql = "SELECT id, table_name, narrative_file, narrative_name, generated_at, \
                   completed_at, row_count, generation_duration_ms, status, error_message, \
                   created_by FROM content_generations ORDER BY generated_at DESC LIMIT $1";
        let (rows, _) = self
            .0
            .query_rows(sql, &[DbValue::Int(limit as i64)])
            .await?;
        rows.rows.iter().map(decode_content_generation).collect()
    }

    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_model_response(&self, record: &ModelResponseRecord) -> BotStorageResult<()> {
        let request_messages = json_str(&record.request_messages)?;
        let response_outputs = json_str(&record.response_outputs)?;
        let sql = "INSERT INTO model_responses \
            (id, created_at, provider, model_name, request_messages, request_temperature, \
             request_max_tokens, request_model, response_outputs, duration_ms, error_message) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
            ON CONFLICT (id) DO NOTHING";
        let params = [
            DbValue::Text(record.id.clone()),
            DbValue::Text(record.created_at.to_rfc3339()),
            DbValue::Text(record.provider.clone()),
            DbValue::Text(record.model_name.clone()),
            DbValue::Text(request_messages),
            opt_float(record.request_temperature.map(|f| f as f64)),
            opt_int(record.request_max_tokens.map(|i| i as i64)),
            opt_text(&record.request_model),
            DbValue::Text(response_outputs),
            opt_int(record.duration_ms.map(|i| i as i64)),
            opt_text(&record.error_message),
        ];
        self.0.execute(sql, &params).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(%limit))]
    async fn list_model_responses(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<ModelResponseRecord>> {
        let sql = "SELECT id, created_at, provider, model_name, request_messages, \
                   request_temperature, request_max_tokens, request_model, response_outputs, \
                   duration_ms, error_message \
                   FROM model_responses ORDER BY created_at DESC LIMIT $1";
        let (rows, _) = self
            .0
            .query_rows(sql, &[DbValue::Int(limit as i64)])
            .await?;
        rows.rows.iter().map(decode_model_response).collect()
    }
}

// ── PostStore ─────────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl PostStore for PostgresStorage {
    #[instrument(skip(self, record), fields(id = %record.id))]
    async fn save_post_history(&self, record: &PostHistoryRecord) -> BotStorageResult<()> {
        let content_json = json_str(&record.content_json)?;
        let sql = "INSERT INTO post_history (id, platform, post_id, content_json, posted_at) \
                   VALUES ($1, $2, $3, $4, $5) \
                   ON CONFLICT (id) DO NOTHING";
        let params = [
            DbValue::Text(record.id.clone()),
            DbValue::Text(record.platform.clone()),
            DbValue::Text(record.post_id.clone()),
            DbValue::Text(content_json),
            DbValue::Text(record.posted_at.to_rfc3339()),
        ];
        self.0.execute(sql, &params).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(%platform, %limit))]
    async fn list_post_history(
        &self,
        platform: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<PostHistoryRecord>> {
        let sql = "SELECT id, platform, post_id, content_json, posted_at \
                   FROM post_history WHERE platform = $1 \
                   ORDER BY posted_at DESC LIMIT $2";
        let (rows, _) = self
            .0
            .query_rows(
                sql,
                &[
                    DbValue::Text(platform.to_owned()),
                    DbValue::Int(limit as i64),
                ],
            )
            .await?;
        rows.rows.iter().map(decode_post_history).collect()
    }
}

// ── BotStorage ────────────────────────────────────────────────────────────────

impl BotStorage for PostgresStorage {}

// ── Row decoders ──────────────────────────────────────────────────────────────

fn decode_narrative_execution(row: &DbRow) -> BotStorageResult<NarrativeExecutionRecord> {
    Ok(NarrativeExecutionRecord {
        id: col_text(row, "id")?,
        narrative_name: col_text(row, "narrative_name")?,
        narrative_description: col_opt_text(row, "narrative_description"),
        started_at: col_dt(row, "started_at")?,
        completed_at: col_opt_dt(row, "completed_at")?,
        status: col_text(row, "status")?,
        error_message: col_opt_text(row, "error_message"),
        created_at: col_dt(row, "created_at")?,
    })
}

fn decode_act_execution(row: &DbRow) -> BotStorageResult<ActExecutionRecord> {
    Ok(ActExecutionRecord {
        id: col_text(row, "id")?,
        narrative_execution_id: col_text(row, "narrative_execution_id")?,
        act_name: col_text(row, "act_name")?,
        sequence_number: col_i32(row, "sequence_number")?,
        model: col_opt_text(row, "model"),
        temperature: col_opt_f32(row, "temperature"),
        max_tokens: col_opt_i32(row, "max_tokens"),
        response: col_text(row, "response")?,
        created_at: col_dt(row, "created_at")?,
    })
}

fn decode_actor_execution(row: &DbRow) -> BotStorageResult<ActorServerExecutionRecord> {
    let metadata_str = col_text(row, "metadata")?;
    let metadata = serde_json::from_str(&metadata_str)?;
    Ok(ActorServerExecutionRecord {
        id: col_text(row, "id")?,
        task_id: col_text(row, "task_id")?,
        actor_name: col_text(row, "actor_name")?,
        started_at: col_dt(row, "started_at")?,
        completed_at: col_opt_dt(row, "completed_at")?,
        success: col_bool(row, "success")?,
        error_message: col_opt_text(row, "error_message"),
        skills_succeeded: col_i32(row, "skills_succeeded")?,
        skills_failed: col_i32(row, "skills_failed")?,
        skills_skipped: col_i32(row, "skills_skipped")?,
        metadata,
        created_at: col_dt(row, "created_at")?,
    })
}

fn decode_actor_state(row: &DbRow) -> BotStorageResult<ActorServerStateRecord> {
    Ok(ActorServerStateRecord {
        task_id: col_text(row, "task_id")?,
        actor_name: col_text(row, "actor_name")?,
        last_run: col_opt_dt(row, "last_run")?,
        next_run: col_dt(row, "next_run")?,
        consecutive_failures: col_i32(row, "consecutive_failures")?,
        is_paused: col_bool(row, "is_paused")?,
        metadata: col_json(row, "metadata")?,
        updated_at: col_dt(row, "updated_at")?,
    })
}

fn decode_content(row: &DbRow) -> BotStorageResult<ContentRecord> {
    Ok(ContentRecord {
        id: col_text(row, "id")?,
        table_name: col_text(row, "table_name")?,
        content_json: col_json(row, "content_json")?,
        created_at: col_dt(row, "created_at")?,
    })
}

fn decode_content_generation(row: &DbRow) -> BotStorageResult<ContentGenerationRecord> {
    Ok(ContentGenerationRecord {
        id: col_text(row, "id")?,
        table_name: col_text(row, "table_name")?,
        narrative_file: col_text(row, "narrative_file")?,
        narrative_name: col_text(row, "narrative_name")?,
        generated_at: col_dt(row, "generated_at")?,
        completed_at: col_opt_dt(row, "completed_at")?,
        row_count: col_opt_i32(row, "row_count"),
        generation_duration_ms: col_opt_i32(row, "generation_duration_ms"),
        status: col_text(row, "status")?,
        error_message: col_opt_text(row, "error_message"),
        created_by: col_opt_text(row, "created_by"),
    })
}

fn decode_model_response(row: &DbRow) -> BotStorageResult<ModelResponseRecord> {
    Ok(ModelResponseRecord {
        id: col_text(row, "id")?,
        created_at: col_dt(row, "created_at")?,
        provider: col_text(row, "provider")?,
        model_name: col_text(row, "model_name")?,
        request_messages: col_json(row, "request_messages")?,
        request_temperature: col_opt_f32(row, "request_temperature"),
        request_max_tokens: col_opt_i32(row, "request_max_tokens"),
        request_model: col_opt_text(row, "request_model"),
        response_outputs: col_json(row, "response_outputs")?,
        duration_ms: col_opt_i32(row, "duration_ms"),
        error_message: col_opt_text(row, "error_message"),
    })
}

fn decode_post_history(row: &DbRow) -> BotStorageResult<PostHistoryRecord> {
    Ok(PostHistoryRecord {
        id: col_text(row, "id")?,
        platform: col_text(row, "platform")?,
        post_id: col_text(row, "post_id")?,
        content_json: col_json(row, "content_json")?,
        posted_at: col_dt(row, "posted_at")?,
    })
}

// ── Parameter helpers ─────────────────────────────────────────────────────────

fn opt_text(v: &Option<String>) -> DbValue {
    match v {
        Some(s) => DbValue::Text(s.clone()),
        None => DbValue::Null,
    }
}

fn opt_int(v: Option<i64>) -> DbValue {
    match v {
        Some(n) => DbValue::Int(n),
        None => DbValue::Null,
    }
}

fn opt_float(v: Option<f64>) -> DbValue {
    match v {
        Some(f) => DbValue::Float(f),
        None => DbValue::Null,
    }
}
