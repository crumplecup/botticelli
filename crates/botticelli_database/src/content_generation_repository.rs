//! Repository for content generation tracking.

use botticelli_error::{DatabaseError, DatabaseErrorKind};
use botticelli_interface::ContentGenerationRepository;
use diesel::prelude::*;
use rmcp::tool;
use tracing::{debug, error};

use crate::{ContentGenerationRow, NewContentGenerationRow, UpdateContentGenerationRow};

/// PostgreSQL implementation of ContentGenerationRepository.
///
/// Uses a mutable reference to PgConnection. For concurrent access,
/// consider wrapping the repository in Arc<Mutex> or using a connection pool.
pub struct PostgresContentGenerationRepository<'a> {
    conn: &'a mut PgConnection,
}

impl<'a> PostgresContentGenerationRepository<'a> {
    /// Create a new repository with a mutable connection reference.
    ///
    /// # Arguments
    /// * `conn` - A mutable reference to a PostgreSQL connection
    ///
    /// # Example
    /// ```no_run
    /// use botticelli_database::{PostgresContentGenerationRepository, establish_connection};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut conn = establish_connection()?;
    /// let repo = PostgresContentGenerationRepository::new(&mut conn);
    /// # Ok(())
    /// # }
    /// ```
    #[tool]
    #[tracing::instrument(skip(conn))]
    pub fn new(conn: &'a mut PgConnection) -> Self {
        Self { conn }
    }
}

impl<'a> ContentGenerationRepository for PostgresContentGenerationRepository<'a> {
    type Row = ContentGenerationRow;
    type NewRow = NewContentGenerationRow;
    type UpdateRow = UpdateContentGenerationRow;
    type Error = DatabaseError;

    #[tracing::instrument(skip(self, new_gen), fields(table = %new_gen.table_name, narrative = ?new_gen.narrative_file))]
    fn start_generation(&mut self, new_gen: Self::NewRow) -> Result<Self::Row, Self::Error> {
        use crate::schema::content_generations;

        debug!(table = %new_gen.table_name, narrative = ?new_gen.narrative_file, "Starting content generation");

        // Check if generation already exists
        if let Some(existing) = self.get_by_table_name(&new_gen.table_name)? {
            debug!(table = %new_gen.table_name, existing_status = %existing.status(), "Generation already exists, returning existing record");
            return Ok(existing);
        }

        let result = diesel::insert_into(content_generations::table)
            .values(&new_gen)
            .get_result(self.conn)
            .map_err(|e| {
                error!(error = %e, table = %new_gen.table_name, "Failed to start content generation");
                DatabaseError::new(DatabaseErrorKind::Query(e.to_string()))
            })?;

        debug!(table = %new_gen.table_name, "Started content generation");
        Ok(result)
    }

    #[tracing::instrument(skip(self, update), fields(table, status = ?update.status))]
    fn complete_generation(
        &mut self,
        table: &str,
        update: Self::UpdateRow,
    ) -> Result<Self::Row, Self::Error> {
        use crate::schema::content_generations::dsl;

        debug!(table = %table, status = ?update.status, "Completing content generation");
        let result = diesel::update(dsl::content_generations.filter(dsl::table_name.eq(table)))
            .set(&update)
            .get_result(self.conn)
            .map_err(|e| {
                error!(error = %e, table = %table, "Failed to complete content generation");
                DatabaseError::new(DatabaseErrorKind::Query(e.to_string()))
            })?;

        debug!(table = %table, "Completed content generation");
        Ok(result)
    }

    #[tracing::instrument(skip(self))]
    fn get_last_successful(&mut self) -> Result<Option<Self::Row>, Self::Error> {
        use crate::schema::content_generations::dsl;

        debug!("Getting last successful generation");
        let result = dsl::content_generations
            .filter(dsl::status.eq("success"))
            .order(dsl::generated_at.desc())
            .first(self.conn)
            .optional()
            .map_err(|e| {
                error!(error = %e, "Failed to get last successful generation");
                DatabaseError::new(DatabaseErrorKind::Query(e.to_string()))
            })?;

        debug!(
            found = result.is_some(),
            "Last successful generation query complete"
        );
        Ok(result)
    }

    #[tracing::instrument(skip(self), fields(status_filter = ?status_filter, limit))]
    fn list_generations(
        &mut self,
        status_filter: Option<String>,
        limit: i64,
    ) -> Result<Vec<Self::Row>, Self::Error> {
        use crate::schema::content_generations::dsl;

        debug!("Listing generations");
        let mut query = dsl::content_generations.into_boxed();

        if let Some(s) = status_filter {
            query = query.filter(dsl::status.eq(s));
        }

        let results = query
            .order(dsl::generated_at.desc())
            .limit(limit)
            .load(self.conn)
            .map_err(|e| {
                error!(error = %e, "Failed to list generations");
                DatabaseError::new(DatabaseErrorKind::Query(e.to_string()))
            })?;

        debug!(count = results.len(), "Listed generations");
        Ok(results)
    }

    #[tracing::instrument(skip(self), fields(table))]
    fn get_by_table_name(&mut self, table: &str) -> Result<Option<Self::Row>, Self::Error> {
        use crate::schema::content_generations::dsl;

        debug!(table = %table, "Getting generation by table name");
        let result = dsl::content_generations
            .filter(dsl::table_name.eq(table))
            .first(self.conn)
            .optional()
            .map_err(|e| {
                error!(error = %e, table = %table, "Failed to get generation by table name");
                DatabaseError::new(DatabaseErrorKind::Query(e.to_string()))
            })?;

        debug!(table = %table, found = result.is_some(), "Get by table name complete");
        Ok(result)
    }

    #[tracing::instrument(skip(self), fields(table))]
    fn delete_generation(&mut self, table: &str) -> Result<(), Self::Error> {
        use crate::schema::content_generations::dsl;

        debug!(table = %table, "Deleting generation");
        diesel::delete(dsl::content_generations.filter(dsl::table_name.eq(table)))
            .execute(self.conn)
            .map(|_| ())
            .map_err(|e| {
                error!(error = %e, table = %table, "Failed to delete generation");
                DatabaseError::new(DatabaseErrorKind::Query(e.to_string()))
            })?;

        debug!(table = %table, "Deleted generation");
        Ok(())
    }
}
