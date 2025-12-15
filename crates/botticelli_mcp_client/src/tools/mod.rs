//! MCP tool implementations for Botticelli.
//!
//! This module provides concrete MCP tools that expose Botticelli's
//! internal capabilities for LLM orchestration.

mod narrative;

use crate::{McpClientResult, ToolRegistry};
use std::sync::Arc;

pub use narrative::{
    CreateNarrativeTool, ListNarrativesTool, LoadNarrativeTool, ValidateNarrativeTool,
};

/// Register all available internal tools into the registry.
///
/// This function registers:
/// - Narrative tools (create, list, load, validate)
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

    Ok(())
}
