//! ContentRepository trait implementation.
//!
//! Provides database-backed implementation of the ContentRepository trait
//! for managing generated content tables.

use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use botticelli_interface::ContentRepository;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use serde_json::Value as JsonValue;

/// Database-backed content repository.
#[derive(Clone)]
pub struct DatabaseContentRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl DatabaseContentRepository {
    /// Create a new content repository with the given connection pool.
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ContentRepository for DatabaseContentRepository {
    async fn list_content(
        &self,
        table_name: &str,
        status_filter: Option<&str>,
        limit: usize,
    ) -> BotticelliResult<Vec<JsonValue>> {
        let table_name = table_name.to_string();
        let status_filter = status_filter.map(|s| s.to_string());
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                botticelli_error::DatabaseError::new(
                    botticelli_error::DatabaseErrorKind::Connection(e.to_string()),
                )
            })?;
            crate::content_management::list_content(
                &mut conn,
                &table_name,
                status_filter.as_deref(),
                limit,
            )
        })
        .await
        .map_err(|e| {
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })?
    }

    async fn update_review_status(
        &self,
        table_name: &str,
        id: i64,
        new_status: &str,
    ) -> BotticelliResult<()> {
        let table_name = table_name.to_string();
        let new_status = new_status.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                botticelli_error::DatabaseError::new(
                    botticelli_error::DatabaseErrorKind::Connection(e.to_string()),
                )
            })?;
            crate::content_management::update_review_status(&mut conn, &table_name, id, &new_status)
        })
        .await
        .map_err(|e| {
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })?
    }

    async fn delete_content(&self, table_name: &str, id: i64) -> BotticelliResult<()> {
        let table_name = table_name.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                botticelli_error::DatabaseError::new(
                    botticelli_error::DatabaseErrorKind::Connection(e.to_string()),
                )
            })?;
            crate::content_management::delete_content(&mut conn, &table_name, id)
        })
        .await
        .map_err(|e| {
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })?
    }
}
