//! Trait interfaces for registry operations.
//!
//! This module defines the trait sandwich pattern for registry interactions,
//! allowing tools to work against trait boundaries while repositories provide
//! concrete implementations.

use crate::{ExecutionFilter, ExecutionStatus, ExecutionSummary, NarrativeExecution};
use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use serde_json::Value;

/// Specialized trait for narrative execution repositories.
#[async_trait]
pub trait NarrativeRegistryOperations: Send + Sync {
    /// Save a narrative execution.
    async fn save_execution(&self, execution: &NarrativeExecution) -> BotticelliResult<i32>;

    /// Load a narrative execution by ID.
    async fn load_execution(&self, id: i32) -> BotticelliResult<NarrativeExecution>;

    /// Update execution status.
    async fn update_status(&self, id: i32, status: ExecutionStatus) -> BotticelliResult<()>;

    /// List executions by filter.
    async fn list_executions(
        &self,
        filter: &ExecutionFilter,
    ) -> BotticelliResult<Vec<ExecutionSummary>>;

    /// Delete an execution.
    async fn delete_execution(&self, id: i32) -> BotticelliResult<()>;
}

/// Specialized trait for database table operations.
#[async_trait]
pub trait DatabaseRegistryOperations: Send + Sync {
    /// Execute a raw SQL query.
    async fn execute_query(&self, query: &str) -> BotticelliResult<Vec<Value>>;

    /// List available tables.
    async fn list_tables(&self) -> BotticelliResult<Vec<String>>;

    /// Get table schema.
    async fn get_schema(&self, table: &str) -> BotticelliResult<Value>;

    /// Query content from a table.
    async fn query_content(
        &self,
        table_name: &str,
        status_filter: Option<&str>,
        limit: i64,
    ) -> BotticelliResult<Vec<Value>>;

    /// Create a new content table.
    async fn create_table(
        &self,
        table_name: &str,
        template_source: &str,
        narrative_file: Option<&str>,
        description: Option<&str>,
    ) -> BotticelliResult<()>;

    /// Check if a table exists.
    async fn table_exists(&self, table_name: &str) -> BotticelliResult<bool>;
}

/// Specialized trait for file-based narrative storage operations.
#[async_trait]
pub trait NarrativeStorageOperations: Send + Sync {
    /// List narrative files in configured directory.
    async fn list_narratives(&self, pattern: Option<&str>) -> BotticelliResult<Vec<String>>;

    /// Load narrative from file by filename.
    async fn load_narrative(&self, filename: &str) -> BotticelliResult<Value>;

    /// Validate narrative structure.
    async fn validate_narrative(&self, toml_content: &str) -> BotticelliResult<Value>;

    /// Parse narrative from TOML content.
    async fn parse_narrative(&self, toml_content: &str, name_override: Option<&str>) -> BotticelliResult<Value>;
}

/// Trait for elicitation (partial narrative) registry operations.
pub trait ElicitationRegistryOperations<T>: Send + Sync {
    /// Get a narrative by ID.
    fn get_narrative(&self, id: &str) -> BotticelliResult<T>;

    /// Update a narrative using a closure.
    fn update_narrative<F>(&self, id: &str, updater: F) -> BotticelliResult<()>
    where
        F: FnOnce(&mut T) -> BotticelliResult<()>;

    /// Remove a narrative from the registry.
    fn remove_narrative(&self, id: &str) -> BotticelliResult<Option<T>>;

    /// Add a narrative to the registry.
    fn add_narrative(&self, narrative: T) -> String;

    /// Get current state/status of a narrative.
    fn get_narrative_state(&self, id: &str) -> BotticelliResult<Value>;

    /// Validate a narrative structure.
    fn validate_narrative(&self, id: &str) -> BotticelliResult<Value>;
    
    /// List all narrative IDs in the registry.
    fn list_narrative_ids(&self) -> Vec<String>;
}
