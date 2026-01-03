//! Content generation repository trait.

/// Repository trait for content generation tracking operations.
///
/// Provides methods to record content generation attempts, query generation history,
/// and manage generation metadata.
pub trait ContentGenerationRepository {
    /// The row type representing a content generation record
    type Row;

    /// The type for creating a new content generation record
    type NewRow;

    /// The type for updating an existing content generation record
    type UpdateRow;

    /// The error type for repository operations
    type Error;

    /// Record the start of a content generation.
    ///
    /// Creates a new tracking record with status='running'.
    ///
    /// # Arguments
    /// * `new_gen` - The generation metadata to record
    ///
    /// # Returns
    /// The created Row with assigned ID
    ///
    /// # Errors
    /// Returns Error if:
    /// - A generation with the same table_name already exists (unique constraint violation)
    /// - Database connection fails
    fn start_generation(&mut self, new_gen: Self::NewRow) -> Result<Self::Row, Self::Error>;

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
    /// The updated Row
    ///
    /// # Errors
    /// Returns Error if:
    /// - No generation with the given table_name exists
    /// - Database connection fails
    fn complete_generation(
        &mut self,
        table_name: &str,
        update: Self::UpdateRow,
    ) -> Result<Self::Row, Self::Error>;

    /// Get the most recently completed successful generation.
    ///
    /// Returns the generation with status='success' and the most recent generated_at timestamp.
    /// Useful for finding the last generated table for display or processing.
    ///
    /// # Returns
    /// Some(Row) if a successful generation exists, None otherwise
    ///
    /// # Errors
    /// Returns Error if database connection fails
    fn get_last_successful(&mut self) -> Result<Option<Self::Row>, Self::Error>;

    /// List generations with optional filtering.
    ///
    /// # Arguments
    /// * `status` - Optional status filter ('running', 'success', 'failed')
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// Vector of Row ordered by generated_at DESC
    ///
    /// # Errors
    /// Returns Error if database connection fails
    fn list_generations(
        &mut self,
        status: Option<String>,
        limit: i64,
    ) -> Result<Vec<Self::Row>, Self::Error>;

    /// Get specific generation by table name.
    ///
    /// # Arguments
    /// * `table_name` - The table name identifying the generation
    ///
    /// # Returns
    /// Some(Row) if found, None otherwise
    ///
    /// # Errors
    /// Returns Error if database connection fails
    fn get_by_table_name(&mut self, table_name: &str) -> Result<Option<Self::Row>, Self::Error>;

    /// Delete generation metadata.
    ///
    /// Removes the tracking record for a generation. Note: this does not delete
    /// the actual content table, only the metadata.
    ///
    /// # Arguments
    /// * `table_name` - The table name identifying the generation
    ///
    /// # Errors
    /// Returns Error if database connection fails
    fn delete_generation(&mut self, table_name: &str) -> Result<(), Self::Error>;
}
