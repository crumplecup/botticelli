//! Knowledge table abstraction for actor data access.

use crate::{ActorError, ActorErrorKind, ActorResult};
use botticelli_interface::BotStorage;
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Wrapper for knowledge table access backed by [`BotStorage`].
///
/// Knowledge tables contain structured data produced by narratives that
/// actors consume during execution.
#[derive(Debug, Clone)]
pub struct KnowledgeTable {
    name: String,
}

impl KnowledgeTable {
    /// Create a new knowledge table reference.
    #[tracing::instrument(skip_all, fields(table_name))]
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        tracing::debug!(table_name = %name, "Creating knowledge table reference");
        Self { name }
    }

    /// Get the table name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Query all rows from the knowledge table.
    ///
    /// # Errors
    ///
    /// Returns error if the storage backend fails.
    #[tracing::instrument(skip(self, storage), fields(table_name = %self.name))]
    pub async fn query(&self, storage: &Arc<dyn BotStorage>) -> ActorResult<Vec<JsonValue>> {
        tracing::debug!("Querying knowledge table");

        let records = storage
            .list_content(&self.name, 10_000)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Knowledge table query failed");
                ActorError::new(ActorErrorKind::KnowledgeTableNotFound(format!(
                    "{}: {}",
                    self.name, e
                )))
            })?;

        tracing::info!(count = records.len(), "Retrieved rows from knowledge table");
        Ok(records.into_iter().map(|r| r.content_json).collect())
    }

    /// Query rows matching a field value.
    ///
    /// Performs an in-memory filter on `field = value`.
    #[tracing::instrument(skip(self, storage), fields(table_name = %self.name, filter))]
    pub async fn query_where(
        &self,
        storage: &Arc<dyn BotStorage>,
        filter: &str,
    ) -> ActorResult<Vec<JsonValue>> {
        tracing::debug!("Querying knowledge table with filter");

        let all = self.query(storage).await?;

        let clause = filter.trim();
        let filtered: Vec<JsonValue> = if let Some((col, val_part)) = clause.split_once(" = ") {
            let col = col.trim();
            let val = val_part.trim().trim_matches('\'');
            all.into_iter()
                .filter(|row| {
                    row.get(col)
                        .and_then(|v| v.as_str())
                        .map(|s| s == val)
                        .unwrap_or(false)
                })
                .collect()
        } else {
            all
        };

        tracing::info!(
            count = filtered.len(),
            "Retrieved filtered rows from knowledge table"
        );
        Ok(filtered)
    }

    /// Get row count from table.
    #[tracing::instrument(skip(self, storage), fields(table_name = %self.name))]
    pub async fn count(&self, storage: &Arc<dyn BotStorage>) -> ActorResult<i64> {
        tracing::debug!("Counting rows in knowledge table");
        let records = storage
            .list_content(&self.name, 10_000)
            .await
            .map_err(|e| {
                ActorError::new(ActorErrorKind::KnowledgeTableNotFound(format!(
                    "{}: {}",
                    self.name, e
                )))
            })?;
        tracing::debug!(count = records.len(), "Row count retrieved");
        Ok(records.len() as i64)
    }

    /// Check if the table has any rows.
    #[tracing::instrument(skip(self, storage), fields(table_name = %self.name))]
    pub async fn exists(&self, storage: &Arc<dyn BotStorage>) -> bool {
        match storage.list_content(&self.name, 1).await {
            Ok(rows) => {
                tracing::debug!(exists = !rows.is_empty(), "Table existence checked");
                !rows.is_empty()
            }
            Err(e) => {
                tracing::warn!(error = ?e, "Failed to check table existence");
                false
            }
        }
    }
}
