//! Tool registration functionality.

use crate::{McpClientResult, ToolRegistry};
use super::{
    CreateCarouselTool, CreateElicitationSessionTool, CreateNarrativeTool, ElicitActTool,
    ElicitMetadataTool, ElicitationRegistry, ExecuteCarouselTool, FinalizeElicitationTool,
    ListNarrativesTool, LoadNarrativeTool, ValidateNarrativeTool,
};
#[cfg(feature = "database")]
use super::{CreateTableTool, InspectTableTool, QueryTableTool, TableExistsTool};
#[cfg(feature = "database")]
use botticelli_database::DbPool;
use std::sync::Arc;

/// Register all available internal tools into the registry.
///
/// This function registers:
/// - Narrative tools (create, list, load, validate)
/// - Elicitation tools (create session, elicit metadata/acts, finalize)
/// - Database tools (create table, query, inspect, exists check - requires `database` feature)
///
/// # Arguments
/// * `registry` - The tool registry to populate
/// * `narratives_dir` - Directory containing narrative TOML files
#[tracing::instrument(skip(registry, narratives_dir))]
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
        Arc::new(CreateCarouselTool::new(elicitation_registry.clone())),
    )?;

    registry.register(
        "execute_carousel".to_string(),
        Arc::new(ExecuteCarouselTool::new(elicitation_registry)),
    )?;

    tracing::info!("Registered all internal tools");
    Ok(())
}

/// Register database tools into the registry.
///
/// Available with the `database` feature.
///
/// # Arguments
/// * `registry` - The tool registry to populate
/// * `db_pool` - Database connection pool
#[cfg(feature = "database")]
#[tracing::instrument(skip(registry, db_pool))]
pub fn register_database_tools(registry: &mut ToolRegistry, db_pool: DbPool) -> McpClientResult<()> {
    let db_ops = DbOperationsImpl::new(db_pool);
    
    registry.register(
        "create_table".to_string(),
        Arc::new(CreateTableTool::new(db_ops.clone())),
    )?;

    registry.register(
        "query_table".to_string(),
        Arc::new(QueryTableTool::new(db_ops.clone())),
    )?;

    registry.register(
        "inspect_table".to_string(),
        Arc::new(InspectTableTool::new(db_ops.clone())),
    )?;

    registry.register(
        "table_exists".to_string(),
        Arc::new(TableExistsTool::new(db_ops)),
    )?;

    tracing::info!("Registered database tools");
    Ok(())
}
