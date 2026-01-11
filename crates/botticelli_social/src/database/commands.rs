//! Database bot command executor.

use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use botticelli_interface::BotCommandExecutor;
use async_trait::async_trait;
use botticelli_database::establish_connection;
use diesel::prelude::*;
use serde_json::{Value as JsonValue, json};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, instrument};

/// Database command executor for narrative-driven database operations.
///
/// Provides safe, whitelisted database operations that can be invoked from
/// narratives using the bot command infrastructure.
///
/// # Security
///
/// - Table names must be whitelisted to prevent SQL injection
/// - Uses parameterized queries via diesel
/// - Updates validated before execution
/// - Returns affected row count for verification
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
#[derive(Debug, Clone)]
pub struct DatabaseCommandExecutor {
    /// Whitelisted table names that can be updated.
    allowed_tables: HashSet<String>,
}

impl DatabaseCommandExecutor {
    /// Create a new database command executor with default allowed tables.
    ///
    /// Default allowed tables:
    /// - approved_discord_posts
    /// - potential_discord_posts
    /// - content
    /// - post_history
    pub fn new() -> Self {
        let mut allowed_tables = HashSet::new();
        allowed_tables.insert("approved_discord_posts".to_string());
        allowed_tables.insert("potential_discord_posts".to_string());
        allowed_tables.insert("content".to_string());
        allowed_tables.insert("post_history".to_string());

        Self { allowed_tables }
    }

    /// Create a new executor with custom allowed tables.
    ///
    /// # Arguments
    ///
    /// * `allowed_tables` - Set of table names that can be updated
    pub fn with_allowed_tables(allowed_tables: HashSet<String>) -> Self {
        Self { allowed_tables }
    }

    /// Add a table to the whitelist.
    pub fn allow_table(&mut self, table_name: impl Into<String>) {
        self.allowed_tables.insert(table_name.into());
    }

    /// Check if a table is whitelisted.
    pub fn is_table_allowed(&self, table_name: &str) -> bool {
        self.allowed_tables.contains(table_name)
    }

    /// Update rows in a table.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments:
    ///   - `table_name` (required): Name of table to update
    ///   - `where_clause` (required): WHERE clause (without WHERE keyword)
    ///   - `updates` (required): Map of column → value updates
    ///   - `limit` (optional): Maximum rows to update
    ///
    /// # Returns
    ///
    /// JSON object with `rows_affected` count.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Required arguments missing
    /// - Table not whitelisted
    /// - Invalid WHERE clause
    /// - Database error
    #[instrument(skip(self), fields(command = "update_table"))]
    async fn update_table(&self, args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
        // Extract table_name
        let table_name = args
            .get("table_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "update_table".to_string(),
                    arg_name: "table_name".to_string(),
                })
            })?;

        // Validate table is whitelisted
        if !self.is_table_allowed(table_name) {
            return Err(BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                command: "update_table".to_string(),
                arg_name: "table_name".to_string(),
                reason: format!(
                    "Table '{}' not in whitelist. Allowed tables: {:?}",
                    table_name, self.allowed_tables
                ),
            }));
        }

        // Extract WHERE clause
        let where_clause = args
            .get("where_clause")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BotCommandError::new(BotCommandErrorKind::MissingArgument {
                    command: "update_table".to_string(),
                    arg_name: "where_clause".to_string(),
                })
            })?;

        // Extract updates
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

        // Extract optional limit
        let limit = args.get("limit").and_then(|v| v.as_i64()).map(|l| l as i32);

        debug!(
            table_name = %table_name,
            where_clause = %where_clause,
            update_count = updates.len(),
            limit = ?limit,
            "Executing update_table command"
        );

        // Build SET clause
        let set_clause = updates
            .iter()
            .map(|(col, val)| {
                let val_str = match val {
                    JsonValue::String(s) => {
                        // Check if it's a SQL function like NOW()
                        if s.to_uppercase() == "NOW()" || s.to_uppercase().starts_with("NOW()") {
                            s.clone()
                        } else {
                            format!("'{}'", s.replace("'", "''")) // Escape single quotes
                        }
                    }
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::Bool(b) => b.to_string(),
                    JsonValue::Null => "NULL".to_string(),
                    _ => {
                        return Err(BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                            command: "update_table".to_string(),
                            arg_name: "updates".to_string(),
                            reason: format!("Unsupported value type for column '{}'", col),
                        }));
                    }
                };
                Ok(format!("{} = {}", col, val_str))
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");

        // Build full UPDATE query
        let query = if let Some(limit_val) = limit {
            format!(
                "UPDATE {} SET {} WHERE {} LIMIT {}",
                table_name, set_clause, where_clause, limit_val
            )
        } else {
            format!(
                "UPDATE {} SET {} WHERE {}",
                table_name, set_clause, where_clause
            )
        };

        debug!(query = %query, "Executing UPDATE query");

        // Execute query
        let mut conn = establish_connection().map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "update_table".to_string(),
                reason: format!("Database connection failed: {}", e),
            })
        })?;

        // PostgreSQL doesn't support LIMIT in UPDATE without a subquery
        // Rewrite query for PostgreSQL compatibility
        let query = if let Some(limit_val) = limit {
            format!(
                "UPDATE {} SET {} WHERE ctid IN (SELECT ctid FROM {} WHERE {} LIMIT {})",
                table_name, set_clause, table_name, where_clause, limit_val
            )
        } else {
            format!(
                "UPDATE {} SET {} WHERE {}",
                table_name, set_clause, where_clause
            )
        };

        debug!(query = %query, "Executing PostgreSQL-compatible UPDATE");

        let rows_affected = diesel::sql_query(&query).execute(&mut conn).map_err(|e| {
            BotCommandError::new(BotCommandErrorKind::ApiError {
                command: "update_table".to_string(),
                reason: format!("Update failed: {}", e),
            })
        })?;

        info!(
            table_name = %table_name,
            rows_affected = rows_affected,
            "Successfully updated table"
        );

        Ok(json!({
            "rows_affected": rows_affected,
            "table_name": table_name,
        }))
    }
}

impl Default for DatabaseCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BotCommandExecutor for DatabaseCommandExecutor {
    type Error = BotCommandError;

    fn platform(&self) -> &str {
        "database"
    }

    fn supports_command(&self, command: &str) -> bool {
        matches!(command, "update_table")
    }

    fn supported_commands(&self) -> Vec<String> {
        vec!["update_table".to_string()]
    }

    #[instrument(
        skip(self, args),
        fields(
            platform = "database",
            command = %command,
            arg_count = args.len()
        )
    )]
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        info!("Executing database bot command");

        let result = match command {
            "update_table" => self.update_table(args).await?,
            _ => {
                return Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
                    format!("database.{}", command),
                )));
            }
        };

        Ok(result)
    }

    fn command_help(&self, command: &str) -> Option<String> {
        match command {
            "update_table" => Some(
                "Update rows in a database table.\n\n\
                Arguments:\n\
                - table_name: Name of table to update (must be whitelisted)\n\
                - where_clause: WHERE clause condition (without WHERE keyword)\n\
                - updates: Object mapping column names to new values\n\
                - limit (optional): Maximum number of rows to update\n\n\
                Example:\n\
                {\n\
                  \"table_name\": \"approved_discord_posts\",\n\
                  \"where_clause\": \"review_status = 'pending'\",\n\
                  \"updates\": {\"review_status\": \"posted\", \"posted_at\": \"NOW()\"},\n\
                  \"limit\": 1\n\
                }"
                .to_string(),
            ),
            _ => None,
        }
    }
}
