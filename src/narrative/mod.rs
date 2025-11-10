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
mod provider;

pub use core::{Narrative, NarrativeMetadata, NarrativeToc};
pub use error::{NarrativeError, NarrativeErrorKind};
pub use executor::{ActExecution, NarrativeExecution, NarrativeExecutor};
pub use provider::{ActConfig, NarrativeProvider};
