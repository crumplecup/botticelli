//! Trait interfaces for registry operations.
//!
//! This module defines the trait sandwich pattern for registry interactions,
//! allowing tools to work against trait boundaries while repositories provide
//! concrete implementations.

use async_trait::async_trait;
use serde_json::Value;

/// Specialized trait for narrative execution repositories.
#[async_trait]
pub trait NarrativeRegistryOperations: Send + Sync {
    /// Error type for operations.
    type Error: std::error::Error + Send + Sync + 'static;
    /// Execution type.
    type Execution;
    /// Execution status type.
    type Status;
    /// Execution filter type.
    type Filter;
    /// Execution summary type.
    type Summary;

    /// Save a narrative execution.
    async fn save_execution(&self, execution: &Self::Execution) -> Result<i32, Self::Error>;

    /// Load a narrative execution by ID.
    async fn load_execution(&self, id: i32) -> Result<Self::Execution, Self::Error>;

    /// Update execution status.
    async fn update_status(&self, id: i32, status: Self::Status) -> Result<(), Self::Error>;

    /// List executions by filter.
    async fn list_executions(
        &self,
        filter: &Self::Filter,
    ) -> Result<Vec<Self::Summary>, Self::Error>;

    /// Delete an execution.
    async fn delete_execution(&self, id: i32) -> Result<(), Self::Error>;
}

/// Specialized trait for database table operations.
#[async_trait]
pub trait DatabaseRegistryOperations: Send + Sync {
    /// Error type for operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute a raw SQL query.
    async fn execute_query(&self, query: &str) -> Result<Vec<Value>, Self::Error>;

    /// List available tables.
    async fn list_tables(&self) -> Result<Vec<String>, Self::Error>;

    /// Get table schema.
    async fn get_schema(&self, table: &str) -> Result<Value, Self::Error>;

    /// Query content from a table.
    async fn query_content(
        &self,
        table_name: &str,
        status_filter: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Value>, Self::Error>;

    /// Create a new content table.
    async fn create_table(
        &self,
        table_name: &str,
        template_source: &str,
        narrative_file: Option<&str>,
        description: Option<&str>,
    ) -> Result<(), Self::Error>;

    /// Check if a table exists.
    async fn table_exists(&self, table_name: &str) -> Result<bool, Self::Error>;
}

/// Specialized trait for file-based narrative storage operations.
#[async_trait]
pub trait NarrativeStorageOperations: Send + Sync {
    /// Error type for operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// List narrative files in configured directory.
    async fn list_narratives(&self, pattern: Option<&str>) -> Result<Vec<String>, Self::Error>;

    /// Load narrative from file by filename.
    async fn load_narrative(&self, filename: &str) -> Result<Value, Self::Error>;

    /// Validate narrative structure.
    async fn validate_narrative(&self, toml_content: &str) -> Result<Value, Self::Error>;

    /// Parse narrative from TOML content.
    async fn parse_narrative(
        &self,
        toml_content: &str,
        name_override: Option<&str>,
    ) -> Result<Value, Self::Error>;
}

/// Trait for elicitation (partial narrative) registry operations.
pub trait ElicitationRegistryOperations<T>: Send + Sync {
    /// Error type for operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get a narrative by ID.
    fn get_narrative(&self, id: &str) -> Result<T, Self::Error>;

    /// Update a narrative using a closure.
    fn update_narrative<F>(&self, id: &str, updater: F) -> Result<(), Self::Error>
    where
        F: FnOnce(&mut T) -> Result<(), Self::Error>;

    /// Remove a narrative from the registry.
    fn remove_narrative(&self, id: &str) -> Result<Option<T>, Self::Error>;

    /// Add a narrative to the registry.
    fn add_narrative(&self, narrative: T) -> String;

    /// Get current state/status of a narrative.
    fn get_narrative_state(&self, id: &str) -> Result<Value, Self::Error>;

    /// Validate a narrative structure.
    fn validate_narrative(&self, id: &str) -> Result<Value, Self::Error>;

    /// List all narrative IDs in the registry.
    fn list_narrative_ids(&self) -> Vec<String>;
}
