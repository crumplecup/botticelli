//! MCP tool implementations for Botticelli.
//!
//! This module provides concrete MCP tools that expose Botticelli's
//! internal capabilities for LLM orchestration.

#[cfg(feature = "database")]
mod database;
// Discord tools temporarily disabled - need proper implementation
// #[cfg(feature = "discord")]
// mod discord;
mod elicitation;
mod narrative;
mod registry;
mod registry_ops;

#[cfg(feature = "database")]
pub use database::{CreateTableTool, InspectTableTool, QueryTableTool, TableExistsTool};
// #[cfg(feature = "discord")]
// pub use discord::{DiscordGetMessagesTool, DiscordSendMessageTool};
pub use elicitation::{
    CreateCarouselTool, CreateElicitationSessionTool, ElicitActTool, ElicitMetadataTool,
    ElicitationRegistry, ExecuteCarouselTool, FinalizeElicitationTool,
};
pub use narrative::{
    CreateNarrativeTool, ListNarrativesTool, LoadNarrativeTool, ValidateNarrativeTool,
};
#[cfg(feature = "database")]
pub use registry::register_database_tools;
pub use registry::register_internal_tools;
pub use registry_ops::{
    GenericRegistry, GetRegistryItemTool, ListRegistryKeysTool, UpsertRegistryItemTool,
};
