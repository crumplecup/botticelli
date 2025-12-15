//! MCP tool implementations for Botticelli.
//!
//! This module provides concrete MCP tools that expose Botticelli's
//! internal capabilities for LLM orchestration.

mod narrative;

pub use narrative::{
    CreateNarrativeTool, ListNarrativesTool, LoadNarrativeTool, ValidateNarrativeTool,
};
