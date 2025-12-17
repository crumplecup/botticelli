//! Tool initialization and registration for MCP orchestrator.
//!
//! This module wires up all Botticelli capabilities as MCP tools:
//! - Narrative generation and management
//! - Elicitation for conversational narrative creation
//! - Database operations
//! - Discord interactions

use botticelli_mcp_client::{
    CreateCarouselTool, CreateElicitationSessionTool, CreateNarrativeTool, ElicitActTool,
    ElicitMetadataTool, ElicitationRegistry, ExecuteCarouselTool,
    FinalizeElicitationTool, ListNarrativesTool, LoadNarrativeTool, ToolRegistry,
    ValidateNarrativeTool,
};
use botticelli_narrative::FilesystemNarrativeStorage;
use std::sync::Arc;

/// Initialize and register all Botticelli tools with the MCP registry.
///
/// This creates a complete tool registry with:
/// - Narrative tools (create, list, load, validate)
/// - Elicitation tools (session management, metadata, acts, carousel, finalize)
#[tracing::instrument(skip(narratives_dir))]
pub fn initialize_tools(narratives_dir: impl Into<String> + std::fmt::Debug) -> Arc<ToolRegistry> {
    let narratives_dir = narratives_dir.into();
    tracing::info!(narratives_dir = %narratives_dir, "Initializing MCP tools");

    let mut registry = ToolRegistry::new();

    // Initialize elicitation registry (shared across elicitation tools)
    let elicitation_registry = ElicitationRegistry::new();

    // Register narrative tools
    register_narrative_tools(&mut registry, &narratives_dir);

    // Register elicitation tools
    register_elicitation_tools(&mut registry, elicitation_registry);

    tracing::info!(
        tool_count = registry.list_tools().len(),
        "MCP tools initialized"
    );

    Arc::new(registry)
}

/// Register all narrative-related tools.
#[tracing::instrument(skip(registry, narratives_dir))]
fn register_narrative_tools(registry: &mut ToolRegistry, narratives_dir: &str) {
    tracing::debug!("Registering narrative tools");

    // Create shared storage instance
    let storage = FilesystemNarrativeStorage::new(narratives_dir.into());

    // Create narrative from TOML
    registry
        .register(
            "create_narrative".to_string(),
            Arc::new(CreateNarrativeTool::new(storage.clone())),
        )
        .expect("Failed to register create_narrative tool");

    // List available narratives
    registry
        .register(
            "list_narratives".to_string(),
            Arc::new(ListNarrativesTool::new(storage.clone())),
        )
        .expect("Failed to register list_narratives tool");

    // Load narrative from file
    registry
        .register(
            "load_narrative".to_string(),
            Arc::new(LoadNarrativeTool::new(storage.clone())),
        )
        .expect("Failed to register load_narrative tool");

    // Validate narrative structure
    registry
        .register(
            "validate_narrative".to_string(),
            Arc::new(ValidateNarrativeTool::new(storage)),
        )
        .expect("Failed to register validate_narrative tool");

    tracing::debug!(count = 4, "Narrative tools registered");
}

/// Register all elicitation-related tools.
#[tracing::instrument(skip(registry, elicitation_registry))]
fn register_elicitation_tools(
    registry: &mut ToolRegistry,
    elicitation_registry: ElicitationRegistry,
) {
    tracing::debug!("Registering elicitation tools");

    // Create elicitation session
    registry
        .register(
            "create_elicitation_session".to_string(),
            Arc::new(CreateElicitationSessionTool::new(
                elicitation_registry.clone(),
            )),
        )
        .expect("Failed to register create_elicitation_session tool");

    // Set/update narrative metadata
    registry
        .register(
            "elicit_metadata".to_string(),
            Arc::new(ElicitMetadataTool::new(elicitation_registry.clone())),
        )
        .expect("Failed to register elicit_metadata tool");

    // Add/update acts
    registry
        .register(
            "elicit_act".to_string(),
            Arc::new(ElicitActTool::new(elicitation_registry.clone())),
        )
        .expect("Failed to register elicit_act tool");

    // Create carousel configuration
    registry
        .register(
            "create_carousel".to_string(),
            Arc::new(CreateCarouselTool::new(elicitation_registry.clone())),
        )
        .expect("Failed to register create_carousel tool");

    // Execute carousel
    registry
        .register(
            "execute_carousel".to_string(),
            Arc::new(ExecuteCarouselTool::new(elicitation_registry.clone())),
        )
        .expect("Failed to register execute_carousel tool");

    // Finalize and generate TOML
    registry
        .register(
            "finalize_elicitation".to_string(),
            Arc::new(FinalizeElicitationTool::new(elicitation_registry)),
        )
        .expect("Failed to register finalize_elicitation tool");

    tracing::debug!(count = 6, "Elicitation tools registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_tools() {
        let registry = initialize_tools("/tmp/narratives");
        let tools = registry.list_tools();

        // Should have all narrative + elicitation tools
        assert!(tools.len() >= 10, "Expected at least 10 tools");

        // Check narrative tools present
        assert!(tools.iter().any(|t| t.name == "create_narrative"));
        assert!(tools.iter().any(|t| t.name == "list_narratives"));
        assert!(tools.iter().any(|t| t.name == "load_narrative"));
        assert!(tools.iter().any(|t| t.name == "validate_narrative"));

        // Check elicitation tools present
        assert!(tools
            .iter()
            .any(|t| t.name == "create_elicitation_session"));
        assert!(tools.iter().any(|t| t.name == "elicit_metadata"));
        assert!(tools.iter().any(|t| t.name == "elicit_act"));
        assert!(tools.iter().any(|t| t.name == "create_carousel"));
        assert!(tools.iter().any(|t| t.name == "execute_carousel"));
        assert!(tools.iter().any(|t| t.name == "finalize_elicitation"));
    }
}
