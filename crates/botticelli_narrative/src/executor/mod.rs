//! Narrative execution logic.
//!
//! This module provides the executor that processes multi-act narratives
//! by calling LLM APIs in sequence, passing context between acts.

mod composition;
mod core;
mod inputs;
mod llm;
mod processing;
mod utils;

pub use core::NarrativeExecutor;

