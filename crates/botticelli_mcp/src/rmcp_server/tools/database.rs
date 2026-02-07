//! Database type elicit tools for MCP server.
//!
//! This module provides elicitation tools for all database types,
//! enabling AI agents to construct, inspect, and validate database
//! operations during sampling and testing.

use crate::rmcp_server::BotticelliServer;
use botticelli_database::{
    // Schema inspection types
    ColumnDefinition, ColumnInfo, InferredSchema, TableSchema,
    // View types for query construction
    TableCountView, TableQueryView,
    // Insert types (New*)
    NewActExecutionRow, NewActInputRow, NewActorServerExecution, NewActorServerState,
    NewContentGenerationRow, NewModelResponse, NewNarrativeExecutionRow,
    // Update types
    UpdateContentGenerationRow,
    // Query result types (Row structs)
    ActExecutionRow, ActInputRow, ActorServerExecutionRow, ActorServerStateRow,
    ContentGenerationRow, ModelResponse, NarrativeExecutionRow,
    // Serializable types
    SerializableModelResponse,
};
use elicitation::Elicit;
use elicitation_macros::elicit_tools;
use rmcp::tool;
use rmcp::tool_router;

// ============================================================================
// Elicit Tools - Separate impl block for database type elicitation
// ============================================================================

#[elicit_tools(
    // Schema inspection types (4)
    ColumnInfo,
    TableSchema,
    ColumnDefinition,
    InferredSchema,
    // View types (2)
    TableQueryView,
    TableCountView,
    // Insert types (7)
    NewModelResponse,
    NewContentGenerationRow,
    NewNarrativeExecutionRow,
    NewActExecutionRow,
    NewActInputRow,
    NewActorServerState,
    NewActorServerExecution,
    // Update types (1)
    UpdateContentGenerationRow,
    // Query result types (7)
    ModelResponse,
    ContentGenerationRow,
    NarrativeExecutionRow,
    ActExecutionRow,
    ActInputRow,
    ActorServerStateRow,
    ActorServerExecutionRow,
    // Serializable types (1)
    SerializableModelResponse
)]
#[tool_router(router = database_elicit_tool_router, vis = "pub(crate)")]
impl BotticelliServer {}
