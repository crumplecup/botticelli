//! MCP tool implementations for Botticelli.
//!
//! This module provides concrete MCP tools that expose Botticelli's
//! internal capabilities for LLM orchestration.

mod elicitation;
mod narrative;
mod registry_ops;

use crate::{McpClientResult, ToolRegistry};
use std::sync::Arc;

pub use elicitation::{
    CreateCarouselTool, CreateElicitationSessionTool, ElicitActTool, ElicitMetadataTool,
    ElicitationRegistry, FinalizeElicitationTool,
};
pub use narrative::{
    CreateNarrativeTool, ListNarrativesTool, LoadNarrativeTool, ValidateNarrativeTool,
};
pub use registry_ops::{
    GenericRegistry, GetRegistryItemTool, ListRegistryKeysTool, UpsertRegistryItemTool,
};

/// Register all available internal tools into the registry.
///
/// This function registers:
/// - Narrative tools (create, list, load, validate)
/// - Elicitation tools (create session, elicit metadata/acts, finalize)
///
/// # Arguments
/// * `registry` - The tool registry to populate
/// * `narratives_dir` - Directory containing narrative TOML files
pub fn register_internal_tools(
    registry: &mut ToolRegistry,
    narratives_dir: impl Into<String>,
) -> McpClientResult<()> {
    let narratives_dir = narratives_dir.into();

    // Register narrative tools
    registry.register(
        "create_narrative".to_string(),
        Arc::new(CreateNarrativeTool),
    )?;

    registry.register(
        "list_narratives".to_string(),
        Arc::new(ListNarrativesTool::new(&narratives_dir)),
    )?;

    registry.register(
        "load_narrative".to_string(),
        Arc::new(LoadNarrativeTool::new(&narratives_dir)),
    )?;

    registry.register(
        "validate_narrative".to_string(),
        Arc::new(ValidateNarrativeTool),
    )?;

    // Register elicitation tools
    let elicitation_registry = ElicitationRegistry::new();

    registry.register(
        "create_elicitation_session".to_string(),
        Arc::new(CreateElicitationSessionTool::new(
            elicitation_registry.clone(),
        )),
    )?;

    registry.register(
        "elicit_metadata".to_string(),
        Arc::new(ElicitMetadataTool::new(elicitation_registry.clone())),
    )?;

    registry.register(
        "elicit_act".to_string(),
        Arc::new(ElicitActTool::new(elicitation_registry.clone())),
    )?;

    registry.register(
        "finalize_elicitation".to_string(),
        Arc::new(FinalizeElicitationTool::new(elicitation_registry.clone())),
    )?;

    registry.register(
        "create_carousel".to_string(),
        Arc::new(CreateCarouselTool::new(elicitation_registry)),
    )?;

    Ok(())
}
