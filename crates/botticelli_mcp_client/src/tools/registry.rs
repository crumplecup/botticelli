//! Tool registration functionality.

use super::{
    CreateCarouselTool, CreateElicitationSessionTool, ElicitActTool, ElicitMetadataTool,
    ElicitationRegistry, ExecuteCarouselTool, FinalizeElicitationTool,
};
#[cfg(feature = "database")]
use super::{CreateTableTool, InspectTableTool, QueryTableTool, TableExistsTool};
use crate::{McpClientResult, ToolRegistry};
#[cfg(feature = "database")]
use botticelli_database::{DbOperationsImpl, DbPool, PostgresNarrativeRepository};
use std::sync::Arc;

/// Register all available internal tools into the registry.
///
/// This function registers:
/// - Narrative tools (create, list, load, validate) - requires `database` feature
/// - Elicitation tools (create session, elicit metadata/acts, finalize)
///
/// # Arguments
/// * `registry` - The tool registry to populate
/// * `narratives_dir` - Directory containing narrative TOML files (unused but kept for API compatibility)
/// * `db_pool` - Optional database pool for narrative tools (required when `database` feature enabled)
#[tracing::instrument(skip(registry, narratives_dir, db_pool))]
pub fn register_internal_tools(
    registry: &mut ToolRegistry,
    narratives_dir: impl Into<String>,
    #[cfg(feature = "database")] db_pool: Option<DbPool>,
) -> McpClientResult<()> {
    let _narratives_dir = narratives_dir.into();

    // Register narrative tools (requires database)
    // TODO: Narrative tools need refactoring to work with database backend properly
    // Currently commented out due to architectural mismatch between file-based trait
    // and database implementation
    #[cfg(feature = "database")]
    {
        let _db_pool = db_pool.ok_or_else(|| {
            crate::McpClientErrorKind::Configuration(
                "Database pool required for narrative tools".to_string(),
            )
        })?;

        // Database operations for database tools
        let db_ops = DbOperationsImpl::new(_db_pool.clone());

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
    }

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
pub fn register_database_tools(
    registry: &mut ToolRegistry,
    db_pool: DbPool,
) -> McpClientResult<()> {
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
