//! Narrative system for multi-step LLM prompt execution.
//!
//! This module provides data structures and functionality for loading and executing
//! multi-act narratives defined in TOML files. Each narrative consists of:
//! - Metadata (name, description)
//! - A table of contents defining execution order
//! - Acts containing prompts to be executed sequentially

mod core;
mod error;
mod executor;
mod extraction;
mod in_memory_repository;
mod processor;
mod provider;
mod repository;
mod toml;

pub use core::{Narrative, NarrativeMetadata, NarrativeToc};
pub use error::{NarrativeError, NarrativeErrorKind};
pub use executor::{ActExecution, NarrativeExecution, NarrativeExecutor};
pub use extraction::{extract_json, extract_toml, parse_json, parse_toml};
pub use in_memory_repository::InMemoryNarrativeRepository;
pub use processor::{ActProcessor, ProcessorRegistry};
pub use provider::{ActConfig, NarrativeProvider};
pub use repository::{
    ExecutionFilter, ExecutionStatus, ExecutionSummary, NarrativeRepository, VideoMetadata,
};
