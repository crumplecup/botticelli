//! Repository for content generation tracking.

use botticelli_error::{DatabaseError, DatabaseErrorKind};
use botticelli_interface::ContentGenerationRepository;
use diesel::prelude::*;
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
    pub fn new(conn: &'a mut PgConnection) -> Self {
        Self { conn }
    }
}

impl<'a> ContentGenerationRepository for PostgresContentGenerationRepository<'a> {
    type Row = ContentGenerationRow;
    type NewRow = NewContentGenerationRow;
    type UpdateRow = UpdateContentGenerationRow;
    type Error = DatabaseError;

    fn start_generation(
        &mut self,
        new_gen: Self::NewRow,
    ) -> Result<Self::Row, Self::Error> {
        use crate::schema::content_generations;

        debug!(table = %new_gen.table_name, narrative = ?new_gen.narrative_file, "Starting content generation");

        // Check if generation already exists
        if let Some(existing) = self.get_by_table_name(&new_gen.table_name)? {
            debug!(table = %new_gen.table_name, existing_status = %existing.status(), "Generation already exists, returning existing record");
            return Ok(existing);
        }

        diesel::insert_into(content_generations::table)
            .values(&new_gen)
            .get_result(self.conn)
            .map_err(|e| {
                error!(error = %e, table = %new_gen.table_name, "Failed to start content generation");
                DatabaseError::new(DatabaseErrorKind::Query(e.to_string()))
            })
    }

    fn complete_generation(
        &mut self,
        table: &str,
        update: Self::UpdateRow,
    ) -> Result<Self::Row, Self::Error> {
        use crate::schema::content_generations::dsl;

        debug!(table = %table, status = ?update.status, "Completing content generation");
        diesel::update(dsl::content_generations.filter(dsl::table_name.eq(table)))
            .set(&update)
            .get_result(self.conn)
            .map_err(|e| {
                error!(error = %e, table = %table, "Failed to complete content generation");
                DatabaseError::new(DatabaseErrorKind::Query(e.to_string()))
            })
    }

    fn get_last_successful(&mut self) -> Result<Option<Self::Row>, Self::Error> {
        use crate::schema::content_generations::dsl;

        dsl::content_generations
            .filter(dsl::status.eq("success"))
            .order(dsl::generated_at.desc())
            .first(self.conn)
            .optional()
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))
    }

    fn list_generations(
        &mut self,
        status_filter: Option<String>,
        limit: i64,
    ) -> Result<Vec<Self::Row>, Self::Error> {
        use crate::schema::content_generations::dsl;

        let mut query = dsl::content_generations.into_boxed();

        if let Some(s) = status_filter {
            query = query.filter(dsl::status.eq(s));
        }

        query
            .order(dsl::generated_at.desc())
            .limit(limit)
            .load(self.conn)
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))
    }

    fn get_by_table_name(&mut self, table: &str) -> Result<Option<Self::Row>, Self::Error> {
        use crate::schema::content_generations::dsl;

        dsl::content_generations
            .filter(dsl::table_name.eq(table))
            .first(self.conn)
            .optional()
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))
    }

    fn delete_generation(&mut self, table: &str) -> Result<(), Self::Error> {
        use crate::schema::content_generations::dsl;

        diesel::delete(dsl::content_generations.filter(dsl::table_name.eq(table)))
            .execute(self.conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))
    }
}
