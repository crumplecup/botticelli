//! Narrative execution tools.
//!
//! Tools for generating content and executing narrative acts.
//!
//! This module is split into several submodules:
//! - `generation`: Simple text generation with LLM drivers
//! - `acts`: Single act execution with context
//! - `narratives`: Full narrative execution from TOML files
//! - `sessions`: Narrative session management

mod acts;
mod generation;
mod helpers;
mod narratives;
mod sessions;

// Re-export all public functions
// The impl blocks in each module extend BotticelliServer, so no explicit re-exports needed
// Rust will find them automatically through the module system
