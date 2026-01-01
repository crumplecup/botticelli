//! ContentRepository trait implementation.
//!
//! Provides database-backed implementation of the ContentRepository trait
//! for managing generated content tables.

use async_trait::async_trait;
use botticelli_interface::ContentRepository;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

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
    type Error = botticelli_error::BotticelliError;

    async fn create_content_table(
        &self,
        table_name: &str,
        schema: &serde_json::Value,
    ) -> Result<String, Self::Error> {
        let table_name = table_name.to_string();
        let schema = schema.clone();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                botticelli_error::DatabaseError::new(
                    botticelli_error::DatabaseErrorKind::Connection(e.to_string()),
                )
            })?;
            crate::content_management::create_content_table(&mut conn, &table_name, &schema)
        })
        .await
        .map_err(|e| {
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })?
    }

    async fn insert_content(
        &self,
        table_name: &str,
        content: &serde_json::Value,
    ) -> Result<i32, Self::Error> {
        let table_name = table_name.to_string();
        let content = content.clone();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                botticelli_error::DatabaseError::new(
                    botticelli_error::DatabaseErrorKind::Connection(e.to_string()),
                )
            })?;
            crate::content_management::insert_content(&mut conn, &table_name, &content)
        })
        .await
        .map_err(|e| {
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })?
    }

    async fn query_content(
        &self,
        table_name: &str,
        filter: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, Self::Error> {
        let table_name = table_name.to_string();
        let filter = filter.map(|s| s.to_string());
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                botticelli_error::DatabaseError::new(
                    botticelli_error::DatabaseErrorKind::Connection(e.to_string()),
                )
            })?;
            crate::content_management::query_content(
                &mut conn,
                &table_name,
                filter.as_deref(),
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
}
