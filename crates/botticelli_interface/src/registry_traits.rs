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
}
