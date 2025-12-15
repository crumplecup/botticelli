//! MCP tool implementations for Botticelli.
//!
//! This module provides concrete MCP tools that expose Botticelli's
//! internal capabilities for LLM orchestration.

#[cfg(feature = "database")]
mod database;
mod elicitation;
mod narrative;
mod registry;
mod registry_ops;

#[cfg(feature = "database")]
pub use database::{CreateTableTool, InspectTableTool, QueryTableTool, TableExistsTool};
pub use elicitation::{
    CreateCarouselTool, CreateElicitationSessionTool, ElicitActTool, ElicitMetadataTool,
    ElicitationRegistry, ExecuteCarouselTool, FinalizeElicitationTool,
};
pub use narrative::{
    CreateNarrativeTool, ListNarrativesTool, LoadNarrativeTool, ValidateNarrativeTool,
};
pub use registry::register_internal_tools;
#[cfg(feature = "database")]
pub use registry::register_database_tools;
pub use registry_ops::{
    GenericRegistry, GetRegistryItemTool, ListRegistryKeysTool, UpsertRegistryItemTool,
};
