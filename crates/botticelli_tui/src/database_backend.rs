//! Database backend implementation for TUI.
//!
//! This module implements the TuiBackend trait using PostgreSQL via Diesel.

use crate::{backend::TuiBackend, ContentRow, TuiError, TuiErrorKind, TuiResult};
use botticelli_database::{
    delete_content, establish_connection, get_content_by_id, list_content,
    update_content_metadata, update_review_status,
};
use diesel::PgConnection;

/// Database backend using PostgreSQL via Diesel.
pub struct DatabaseBackend {
    connection: PgConnection,
}

impl DatabaseBackend {
    /// Create a new database backend.
    ///
    /// Connects to PostgreSQL using DATABASE_URL environment variable.
    pub fn new() -> TuiResult<Self> {
        let connection = establish_connection().map_err(|e| {
            TuiError::new(TuiErrorKind::Database(format!(
                "Failed to connect to database: {}",
                e
            )))
        })?;
        
        Ok(Self { connection })
    }
}

impl TuiBackend for DatabaseBackend {
    fn list_content(&mut self, table_name: &str, limit: i64) -> TuiResult<Vec<ContentRow>> {
        let rows = list_content(&mut self.connection, table_name, None, limit as usize).map_err(
            |e| TuiError::new(TuiErrorKind::Database(format!("Failed to list content: {}", e))),
        )?;
        
        let content_rows = rows
            .into_iter()
            .map(|row| {
                let id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                let review_status = row
                    .get("review_status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending")
                    .to_string();
                let rating = row.get("rating").and_then(|v| v.as_i64()).map(|v| v as i32);
                let tags = row
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                
                let content = row.get("content").cloned().unwrap_or(serde_json::Value::Null);
                let preview = content
                    .as_str()
                    .map(|s| s.chars().take(50).collect())
                    .unwrap_or_else(|| content.to_string().chars().take(50).collect());
                
                let source_narrative = row
                    .get("source_narrative")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let source_act = row.get("source_act").and_then(|v| v.as_str()).map(String::from);
                
                ContentRow {
                    id,
                    review_status,
                    rating,
                    tags,
                    preview,
                    content,
                    source_narrative,
                    source_act,
                }
            })
            .collect();
        
        Ok(content_rows)
    }

    fn update_metadata(
        &mut self,
        table_name: &str,
        id: i64,
        tags: &[String],
        rating: Option<i32>,
        status: &str,
    ) -> TuiResult<()> {
        // Update tags and rating
        update_content_metadata(&mut self.connection, table_name, id, Some(tags), rating).map_err(
            |e| {
                TuiError::new(TuiErrorKind::Database(format!(
                    "Failed to update metadata: {}",
                    e
                )))
            },
        )?;
        
        // Update review status separately
        update_review_status(&mut self.connection, table_name, id, status).map_err(|e| {
            TuiError::new(TuiErrorKind::Database(format!(
                "Failed to update review status: {}",
                e
            )))
        })?;
        
        Ok(())
    }

    fn delete_item(&mut self, table_name: &str, id: i64) -> TuiResult<()> {
        delete_content(&mut self.connection, table_name, id).map_err(|e| {
            TuiError::new(TuiErrorKind::Database(format!("Failed to delete item: {}", e)))
        })?;
        
        Ok(())
    }

    fn export_items(&mut self, table_name: &str, ids: &[i64]) -> TuiResult<String> {
        // Fetch items and convert to JSON
        let items: Result<Vec<_>, _> = ids
            .iter()
            .map(|&id| get_content_by_id(&mut self.connection, table_name, id))
            .collect();
        
        let items = items.map_err(|e| {
            TuiError::new(TuiErrorKind::Database(format!(
                "Failed to fetch items for export: {}",
                e
            )))
        })?;
        
        serde_json::to_string_pretty(&items).map_err(|e| {
            TuiError::new(TuiErrorKind::Database(format!("Failed to serialize items: {}", e)))
        })
    }
}
