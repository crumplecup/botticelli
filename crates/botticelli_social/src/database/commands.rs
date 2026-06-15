//! Database bot command executor backed by BotStorage.

use crate::{BotCommandError, BotCommandErrorKind, BotCommandExecutor, BotCommandResult};
use async_trait::async_trait;
use botticelli_interface::{BotStorage, ContentRecord};
use serde_json::{Value as JsonValue, json};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// KV-backed command executor for narrative-driven table update operations.
///
/// Provides safe, whitelisted update operations on content tables managed
/// by `BotStorage`. Only tables explicitly allowlisted can be updated.
///
/// # Security
///
/// - Table names must be whitelisted to prevent arbitrary writes
/// - WHERE clause matching is done in Rust (simple `col = 'val'` comparison)
/// - Complex SQL is not supported; only JSON-key equality
///
/// # Example
///
/// ```toml
/// [bots.mark_posted]
/// platform = "database"
/// command = "update_table"
/// table_name = "approved_discord_posts"
/// where_clause = "review_status = 'pending'"
/// limit = 1
///
/// [bots.mark_posted.updates]
/// review_status = "posted"
/// posted_at = "NOW()"
/// ```
pub struct DatabaseCommandExecutor {
    storage: Arc<dyn BotStorage>,
    /// Whitelisted table names that can be updated.
    allowed_tables: HashSet<String>,
}

impl std::fmt::Debug for DatabaseCommandExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseCommandExecutor")
            .field("allowed_tables", &self.allowed_tables)
            .finish_non_exhaustive()
    }
}

impl Clone for DatabaseCommandExecutor {
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
            allowed_tables: self.allowed_tables.clone(),
        }
    }
}

impl DatabaseCommandExecutor {
    /// Create a new database command executor with default allowed tables.
    ///
    /// Default allowed tables:
    /// - approved_discord_posts
    /// - potential_discord_posts
    /// - content
    /// - post_history
    pub fn new(storage: Arc<dyn BotStorage>) -> Self {
        let mut allowed_tables = HashSet::new();
        allowed_tables.insert("approved_discord_posts".to_string());
        allowed_tables.insert("potential_discord_posts".to_string());
        allowed_tables.insert("content".to_string());
        allowed_tables.insert("post_history".to_string());

        Self { storage, allowed_tables }
    }

    /// Create a new executor with custom allowed tables.
    pub fn with_allowed_tables(storage: Arc<dyn BotStorage>, allowed_tables: HashSet<String>) -> Self {
        Self { storage, allowed_tables }
    }

    /// Add a table to the whitelist.
    pub fn allow_table(&mut self, table_name: impl Into<String>) {
        self.allowed_tables.insert(table_name.into());
    }

    /// Check if a table is whitelisted.
    pub fn is_table_allowed(&self, table_name: &str) -> bool {
        self.allowed_tables.contains(table_name)
    }

    /// Check whether a content record's JSON matches a simple WHERE clause.
    ///
    /// Supports `col = 'value'` (string) and `col = number` comparisons only.
    #[instrument(skip(json))]
    fn matches_where(json: &JsonValue, where_clause: &str) -> bool {
        let clause = where_clause.trim();
        if let Some((col, val_part)) = clause.split_once(" = ") {
            let col = col.trim();
            let val = val_part.trim();
            if val.starts_with('\'') && val.ends_with('\'') {
                let expected = &val[1..val.len() - 1];
                return json.get(col).and_then(|v| v.as_str()) == Some(expected);
            }
            if let Ok(n) = val.parse::<i64>() {
                return json.get(col).and_then(|v| v.as_i64()) == Some(n);
            }
        }
        false
    }

    /// Apply update key-value pairs to a JSON object.
    ///
    /// Recognises `"NOW()"` as a sentinel for the current UTC timestamp.
    #[instrument(skip(obj, updates))]
    fn apply_updates(
        obj: &mut serde_json::Map<String, JsonValue>,
        updates: &serde_json::Map<String, JsonValue>,
    ) {
        for (key, val) in updates {
            let new_val = if let JsonValue::String(s) = val {
                if s.eq_ignore_ascii_case("NOW()") {
                    JsonValue::String(chrono::Utc::now().to_rfc3339())
                } else {
                    val.clone()
                }
            } else {
                val.clone()
            };
            obj.insert(key.clone(), new_val);
        }
    }

    /// Update rows in a content table matching a WHERE clause.
    #[instrument(skip(self, args), fields(command = "update_table"))]
    async fn update_table(&self, args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
        let table_name = args
            .get("table_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "update_table".to_string(),
                    arg_name: "table_name".to_string(),
                })
            })?;

        if !self.is_table_allowed(table_name) {
            return Err(BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "update_table".to_string(),
                arg_name: "table_name".to_string(),
                reason: format!(
                    "Table '{}' not in whitelist. Allowed: {:?}",
                    table_name, self.allowed_tables
                ),
            }));
        }

        let where_clause = args
            .get("where_clause")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "update_table".to_string(),
                    arg_name: "where_clause".to_string(),
                })
            })?;

        let updates = args
            .get("updates")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "update_table".to_string(),
                    arg_name: "updates".to_string(),
                })
            })?;

        if updates.is_empty() {
            return Err(BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "update_table".to_string(),
                arg_name: "updates".to_string(),
                reason: "Updates map cannot be empty".to_string(),
            }));
        }

        let limit = args.get("limit").and_then(|v| v.as_i64()).map(|l| l as usize);

        debug!(
            table_name = %table_name,
            where_clause = %where_clause,
            update_count = updates.len(),
            limit = ?limit,
            "Executing update_table command"
        );

        let records = self
            .storage
            .list_content(table_name, 10_000)
            .await
            .map_err(|e| {
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: "update_table".to_string(),
                    reason: format!("Storage read failed: {}", e),
                })
            })?;

        let mut rows_affected = 0usize;

        for record in records {
            if !Self::matches_where(&record.content_json, where_clause) {
                continue;
            }

            let mut new_json = record.content_json.clone();
            if let Some(obj) = new_json.as_object_mut() {
                Self::apply_updates(obj, updates);
            }

            let updated = ContentRecord {
                id: record.id,
                table_name: record.table_name,
                content_json: new_json,
                created_at: record.created_at,
            };

            self.storage
                .save_content(&updated)
                .await
                .map_err(|e| {
                    BotCommandError::new(BotCommandErrorKind::ApiError {
                        command: "update_table".to_string(),
                        reason: format!("Storage write failed: {}", e),
                    })
                })?;

            rows_affected += 1;
            if limit.is_some_and(|l| rows_affected >= l) {
                break;
            }
        }

        info!(
            table_name = %table_name,
            rows_affected = rows_affected,
            "update_table completed"
        );

        Ok(json!({
            "rows_affected": rows_affected,
            "table_name": table_name,
        }))
    }
}

#[async_trait]
impl BotCommandExecutor for DatabaseCommandExecutor {
    #[instrument(skip(self))]
    fn platform(&self) -> &str {
        "database"
    }

    #[instrument(skip(self))]
    fn supports_command(&self, command: &str) -> bool {
        matches!(command, "update_table")
    }

    #[instrument(skip(self))]
    fn supported_commands(&self) -> Vec<String> {
        vec!["update_table".to_string()]
    }

    #[instrument(skip(self, args), fields(platform = "database", command = %command, arg_count = args.len()))]
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        info!("Executing database bot command");

        match command {
            "update_table" => self.update_table(args).await,
            _ => Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
                format!("database.{}", command),
            ))),
        }
    }

    async fn messages_bulk_delete(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.messages_bulk_delete".to_string(),
        )))
    }

    async fn threads_create(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.threads_create".to_string(),
        )))
    }

    async fn threads_list(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.threads_list".to_string(),
        )))
    }

    async fn threads_get(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.threads_get".to_string(),
        )))
    }

    async fn threads_edit(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.threads_edit".to_string(),
        )))
    }

    async fn threads_delete(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.threads_delete".to_string(),
        )))
    }

    async fn threads_join(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.threads_join".to_string(),
        )))
    }

    async fn threads_leave(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.threads_leave".to_string(),
        )))
    }

    async fn threads_add_member(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.threads_add_member".to_string(),
        )))
    }

    async fn threads_remove_member(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.threads_remove_member".to_string(),
        )))
    }

    async fn reactions_list(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.reactions_list".to_string(),
        )))
    }

    async fn reactions_clear(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.reactions_clear".to_string(),
        )))
    }

    async fn reactions_clear_emoji(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
            "database.reactions_clear_emoji".to_string(),
        )))
    }

    #[instrument(skip(self))]
    fn command_help(&self, command: &str) -> Option<String> {
        match command {
            "update_table" => Some(
                "Update rows in a content table.\n\n\
                Arguments:\n\
                - table_name: Name of table to update (must be whitelisted)\n\
                - where_clause: Simple equality condition (col = 'val' or col = number)\n\
                - updates: Object mapping field names to new values\n\
                - limit (optional): Maximum number of rows to update"
                    .to_string(),
            ),
            _ => None,
        }
    }
}
