//! ContentRepository trait implementation.
//!
//! Provides database-backed implementation of the ContentRepository trait
//! for managing generated content tables.

use async_trait::async_trait;
use botticelli_interface::ContentRepository;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use rmcp::tool;

/// Database-backed content repository.
#[derive(Clone)]
pub struct DatabaseContentRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl DatabaseContentRepository {
    /// Create a new content repository with the given connection pool.
    #[tool]
    #[tracing::instrument(skip(pool))]
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    /// Get a reference to the connection pool.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn pool(&self) -> &Pool<ConnectionManager<PgConnection>> {
        &self.pool
    }
}

#[async_trait]
impl ContentRepository for DatabaseContentRepository {
    type Error = botticelli_error::BotticelliError;

    #[tracing::instrument(skip(self, schema), fields(table_name, has_template = schema.get("template_source").is_some()))]
    async fn create_content_table(
        &self,
        table_name: &str,
        schema: &serde_json::Value,
    ) -> Result<String, Self::Error> {
        tracing::debug!("Creating content table");
        let table_name_owned = table_name.to_string();
        let schema = schema.clone();
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(
            move || -> Result<String, botticelli_error::DatabaseError> {
                let mut conn = pool.get().map_err(|e| {
                    tracing::error!(error = %e, "Failed to get connection from pool");
                    botticelli_error::DatabaseError::new(
                        botticelli_error::DatabaseErrorKind::Connection(e.to_string()),
                    )
                })?;

                // Extract fields from schema
                let template_source = schema
                    .get("template_source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        tracing::error!("Schema missing template_source field");
                        botticelli_error::DatabaseError::new(
                            botticelli_error::DatabaseErrorKind::Query(
                                "Schema must contain 'template_source' field".to_string(),
                            ),
                        )
                    })?;

                let narrative_file = schema.get("narrative_file").and_then(|v| v.as_str());
                let description = schema.get("description").and_then(|v| v.as_str());

                crate::schema_reflection::create_content_table(
                    &mut conn,
                    &table_name_owned,
                    template_source,
                    narrative_file,
                    description,
                )?;

                Ok(format!("Table '{}' created successfully", table_name_owned))
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Task failed");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })??;

        tracing::info!(table_name, "Created content table");
        Ok(result)
    }

    #[tracing::instrument(skip(self, content), fields(table_name, content_fields = content.as_object().map(|o| o.len())))]
    async fn insert_content(
        &self,
        table_name: &str,
        content: &serde_json::Value,
    ) -> Result<i32, Self::Error> {
        tracing::debug!("Inserting content");
        let table_name_owned = table_name.to_string();
        let content = content.clone();
        let pool = self.pool.clone();

        let id = tokio::task::spawn_blocking(
            move || -> Result<i32, botticelli_error::BotticelliError> {
                let mut conn = pool.get().map_err(|e| {
                    tracing::error!(error = %e, "Failed to get connection from pool");
                    botticelli_error::BotticelliError::from(botticelli_error::DatabaseError::new(
                        botticelli_error::DatabaseErrorKind::Connection(e.to_string()),
                    ))
                })?;
                crate::content_management::insert_content(&mut conn, &table_name_owned, &content)
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Task failed");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })??;

        tracing::info!(table_name, id, "Inserted content");
        Ok(id)
    }

    #[tracing::instrument(skip(self), fields(table_name, filter = ?filter, limit = ?limit))]
    async fn query_content(
        &self,
        table_name: &str,
        filter: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, Self::Error> {
        tracing::debug!("Querying content");
        let table_name_owned = table_name.to_string();
        let filter = filter.map(|s| s.to_string());
        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(
            move || -> Result<Vec<serde_json::Value>, botticelli_error::BotticelliError> {
                let mut conn = pool.get().map_err(|e| {
                    tracing::error!(error = %e, "Failed to get connection from pool");
                    botticelli_error::BotticelliError::from(botticelli_error::DatabaseError::new(
                        botticelli_error::DatabaseErrorKind::Connection(e.to_string()),
                    ))
                })?;
                crate::content_management::query_content(
                    &mut conn,
                    &table_name_owned,
                    filter.as_deref(),
                    limit,
                )
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Task failed");
            botticelli_error::DatabaseError::new(botticelli_error::DatabaseErrorKind::Query(
                e.to_string(),
            ))
        })??;

        tracing::info!(table_name, count = results.len(), "Queried content");
        Ok(results)
    }
}
