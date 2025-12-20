//! Narrative execution types and repository trait.
//!
//! This module provides the core types and interfaces for narrative execution
//! and persistence, shared between executor and database implementations.

pub mod execution;
pub mod repository;

pub use execution::{ActExecution, ActExecutionBuilder, NarrativeExecution};
pub use repository::{ExecutionFilter, ExecutionStatus, ExecutionSummary, NarrativeRepository};
