//! Repository for content generation tracking.

use crate::{
    ContentGenerationRow, DatabaseResult, DatabaseError, DatabaseErrorKind,
    NewContentGenerationRow, UpdateContentGenerationRow,
};
use diesel::prelude::*;

/// Repository trait for content generation tracking operations.
///
/// Provides methods to record content generation attempts, query generation history,
/// and manage generation metadata.
pub trait ContentGenerationRepository {
    /// Record the start of a content generation.
    ///
    /// Creates a new tracking record with status='running'.
    ///
    /// # Arguments
    /// * `new_gen` - The generation metadata to record
    ///
    /// # Returns
    /// The created ContentGenerationRow with assigned ID
    ///
    /// # Errors
    /// Returns DatabaseError if:
    /// - A generation with the same table_name already exists (unique constraint violation)
    /// - Database connection fails
    fn start_generation(
        &mut self,
        new_gen: NewContentGenerationRow,
    ) -> DatabaseResult<ContentGenerationRow>;

    /// Update generation status on completion.
    ///
    /// Updates the generation record identified by table_name with completion metadata.
    /// Typically used to set status='success' or status='failed'.
    ///
    /// # Arguments
    /// * `table_name` - The table name identifying the generation
    /// * `update` - The completion metadata to record
    ///
    /// # Returns
    /// The updated ContentGenerationRow
    ///
    /// # Errors
    /// Returns DatabaseError if:
    /// - No generation with the given table_name exists
    /// - Database connection fails
    fn complete_generation(
        &mut self,
        table_name: &str,
        update: UpdateContentGenerationRow,
    ) -> DatabaseResult<ContentGenerationRow>;

    /// Get the most recently completed successful generation.
    ///
    /// Returns the generation with status='success' and the most recent generated_at timestamp.
    /// Useful for finding the last generated table for display or processing.
    ///
    /// # Returns
    /// Some(ContentGenerationRow) if a successful generation exists, None otherwise
    ///
    /// # Errors
    /// Returns DatabaseError if database connection fails
    fn get_last_successful(&mut self) -> DatabaseResult<Option<ContentGenerationRow>>;

    /// List generations with optional filtering.
    ///
    /// # Arguments
    /// * `status` - Optional status filter ('running', 'success', 'failed')
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// Vector of ContentGenerationRow ordered by generated_at DESC
    ///
    /// # Errors
    /// Returns DatabaseError if database connection fails
    fn list_generations(
        &mut self,
        status: Option<String>,
        limit: i64,
    ) -> DatabaseResult<Vec<ContentGenerationRow>>;

    /// Get specific generation by table name.
    ///
    /// # Arguments
    /// * `table_name` - The table name identifying the generation
    ///
    /// # Returns
    /// Some(ContentGenerationRow) if found, None otherwise
    ///
    /// # Errors
    /// Returns DatabaseError if database connection fails
    fn get_by_table_name(
        &mut self,
        table_name: &str,
    ) -> DatabaseResult<Option<ContentGenerationRow>>;

    /// Delete generation metadata.
    ///
    /// Removes the tracking record for a generation. Note: this does not delete
    /// the actual content table, only the metadata.
    ///
    /// # Arguments
    /// * `table_name` - The table name identifying the generation
    ///
    /// # Errors
    /// Returns DatabaseError if database connection fails
    fn delete_generation(&mut self, table_name: &str) -> DatabaseResult<()>;
}

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
    /// use botticelli::{PostgresContentGenerationRepository, establish_connection};
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
    fn start_generation(
        &mut self,
        new_gen: NewContentGenerationRow,
    ) -> DatabaseResult<ContentGenerationRow> {
        use crate::schema::content_generations;

        diesel::insert_into(content_generations::table)
            .values(&new_gen)
            .get_result(self.conn)
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))
    }

    fn complete_generation(
        &mut self,
        table: &str,
        update: UpdateContentGenerationRow,
    ) -> DatabaseResult<ContentGenerationRow> {
        use crate::schema::content_generations::dsl;

        diesel::update(dsl::content_generations.filter(dsl::table_name.eq(table)))
            .set(&update)
            .get_result(self.conn)
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))
    }

    fn get_last_successful(&mut self) -> DatabaseResult<Option<ContentGenerationRow>> {
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
    ) -> DatabaseResult<Vec<ContentGenerationRow>> {
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

    fn get_by_table_name(&mut self, table: &str) -> DatabaseResult<Option<ContentGenerationRow>> {
        use crate::schema::content_generations::dsl;

        dsl::content_generations
            .filter(dsl::table_name.eq(table))
            .first(self.conn)
            .optional()
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))
    }

    fn delete_generation(&mut self, table: &str) -> DatabaseResult<()> {
        use crate::schema::content_generations::dsl;

        diesel::delete(dsl::content_generations.filter(dsl::table_name.eq(table)))
            .execute(self.conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::new(DatabaseErrorKind::Query(e.to_string())))
    }
}
