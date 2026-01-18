//! Tool implementations for MCP server.

mod bot_commands;
mod elicitation;
mod metrics;
mod narrative_creation;
mod narrative_processor;
pub mod narrative_utils;
pub mod narrative_validation_helpers;
mod prometheus;
mod sampling;
mod sampling_session_manager;

pub use bot_commands::{BotCommandRequest, BotCommandResponse};
// All tool structs migrated to rmcp - tools now accessed via ToolRegistry delegation
pub use elicitation::{
    ApplyValidationFixesInput, ApplyValidationFixesOutput,
    ElicitationHelper,
    GetNarrativeStateInput, GetNarrativeStateOutput, NarrativeRegistry,
    PartialNarrativeRegistry, ValidateNarrativeInput, ValidateNarrativeOutput,
};
pub use metrics::{ActMetrics, ExecutionMetrics};
pub use narrative_creation::{
    ElicitActInput, ElicitMetadataInput, FinalizeNarrativeInput, StartNarrativeInput,
    StartNarrativeTool,
};
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
pub use narrative_processor::McpProcessorCollector;
pub use prometheus::{MetricsSummary, PrometheusMetrics};

// Export shared narrative utilities
pub use narrative_utils::{Act, NarrativeHelper};
pub use sampling::{SamplingCoordinator, SamplingHelper, SamplingResult};
pub use sampling_session_manager::SamplingSessionManager;

// Re-export LlmSamplerOperations for convenience
pub use botticelli_interface::LlmSamplerOperations;

// scene module is now empty - all scene tools migrated to rmcp

use botticelli_error::{McpError, McpResult};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, instrument, trace, warn};

/// Registry for managing MCP tools.
///
/// Delegates tool execution to the rmcp BotticelliServer.
#[derive(Clone)]
pub struct ToolRegistry {
    server: Arc<crate::BotticelliServer>,
}

impl ToolRegistry {
    /// Creates a new tool registry that delegates to the given server.
    pub fn new(server: Arc<crate::BotticelliServer>) -> Self {
        Self { server }
    }

    /// Get tool definitions for LLM function calling.
    ///
    /// Retrieves all registered tools from the rmcp ToolRouter.
    #[instrument(skip(self))]
    pub fn tool_definitions(&self) -> Vec<botticelli_core::ToolDefinition> {
        debug!("Retrieving tool definitions from rmcp ToolRouter");
        
        let tools = self.server.tool_router.list_all()
            .into_iter()
            .map(|tool| {
                trace!(
                    tool_name = %tool.name,
                    has_description = tool.description.is_some(),
                    "Converting tool info to ToolDefinition"
                );
                
                botticelli_core::ToolDefinition::new(
                    tool.name.into_owned(),
                    tool.description.map(|d| d.into_owned()).unwrap_or_default(),
                    serde_json::to_value(&*tool.input_schema).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        
        debug!(tool_count = tools.len(), "Retrieved tool definitions");
        tools
    }

    /// Executes a tool by name, delegating to rmcp handlers.
    #[instrument(skip(self, input), fields(tool_name = name))]
    pub async fn execute(&self, name: &str, input: Value) -> McpResult<Value> {
        debug!(tool_name = name, "Executing tool via rmcp delegation");
        
        // Delegate to rmcp handler based on tool name
        match name {
            // Core tools
            "echo" => {
                debug!("Delegating to echo handler");
                self.server.echo(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "server_info" => {
                debug!("Delegating to server_info handler");
                self.server.server_info()
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            // Narrative tools
            "create_narrative" => {
                debug!("Delegating to create_narrative handler");
                self.server.create_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "modify_narrative" => {
                debug!("Delegating to modify_narrative handler");
                self.server.modify_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "save_narrative" => {
                debug!("Delegating to save_narrative handler");
                self.server.save_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "validate_narrative" => {
                debug!("Delegating to validate_narrative handler");
                self.server.validate_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            // Execution tools
            "execute_act" => {
                debug!("Delegating to execute_act handler");
                self.server.execute_act(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "execute_narrative" => {
                debug!("Delegating to execute_narrative handler");
                self.server.execute_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "generate" => {
                debug!("Delegating to generate handler");
                self.server.generate(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            // Elicitation session tools
            "create_narrative_session" => {
                debug!("Delegating to create_narrative_session handler");
                self.server.create_narrative_session(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "elicit_metadata" => {
                debug!("Delegating to elicit_metadata handler");
                self.server.elicit_metadata(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "elicit_act" => {
                debug!("Delegating to elicit_act handler");
                self.server.elicit_act(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "finalize_narrative" => {
                debug!("Delegating to finalize_narrative handler");
                self.server.finalize_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "elicit_carousel" => {
                debug!("Delegating to elicit_carousel handler");
                self.server.elicit_carousel(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "get_narrative_state" => {
                debug!("Delegating to get_narrative_state handler");
                self.server.get_narrative_state(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "validate_narrative_session" => {
                debug!("Delegating to validate_narrative_session handler");
                self.server.validate_narrative_session(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "apply_validation_fixes" => {
                debug!("Delegating to apply_validation_fixes handler");
                self.server.apply_validation_fixes(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            // Elicitation primitive tools
            "elicit_text" => {
                debug!("Delegating to elicit_text handler");
                self.server.elicit_text(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "elicit_bool" => {
                debug!("Delegating to elicit_bool handler");
                self.server.elicit_bool(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "elicit_number" => {
                debug!("Delegating to elicit_number handler");
                self.server.elicit_number(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            "elicit_select" => {
                debug!("Delegating to elicit_select handler");
                self.server.elicit_select(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            // Scene management tools (feature-gated)
            #[cfg(feature = "database")]
            "create_scene" => {
                debug!("Delegating to create_scene handler");
                self.server.create_scene(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            #[cfg(feature = "database")]
            "list_scenes" => {
                debug!("Delegating to list_scenes handler");
                self.server.list_scenes(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            #[cfg(feature = "database")]
            "update_scene" => {
                debug!("Delegating to update_scene handler");
                self.server.update_scene(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            #[cfg(feature = "database")]
            "delete_scene" => {
                debug!("Delegating to delete_scene handler");
                self.server.delete_scene(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            // Discord tools (feature-gated)
            #[cfg(feature = "discord")]
            "discord_post_message" => {
                debug!("Delegating to discord_post_message handler");
                self.server.discord_post_message(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            #[cfg(feature = "discord")]
            "discord_get_messages" => {
                debug!("Delegating to discord_get_messages handler");
                self.server.discord_get_messages(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            #[cfg(feature = "discord")]
            "discord_get_guild_info" => {
                debug!("Delegating to discord_get_guild_info handler");
                self.server.discord_get_guild_info(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            #[cfg(feature = "discord")]
            "discord_get_channels" => {
                debug!("Delegating to discord_get_channels handler");
                self.server.discord_get_channels(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            // Database tool (feature-gated)
            #[cfg(feature = "database")]
            "query_content" => {
                debug!("Delegating to query_content handler");
                self.server.query_content(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            // Metrics tool
            "export_metrics" => {
                debug!("Delegating to export_metrics handler");
                self.server.export_metrics(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)
                        .map_err(|e| McpError::invalid_input(e.to_string()))?,
                ))
                .await
                .map(|json| serde_json::to_value(json.0).unwrap())
                .map_err(|e| McpError::execution_failed(e.message.to_string()))
            }
            
            _ => {
                warn!(tool_name = name, "Unknown tool requested");
                Err(McpError::tool_not_found(name))
            }
        }
    }
}

/// Returns the number of registered tools.
impl ToolRegistry {
    /// Gets the number of registered tools.
    pub fn len(&self) -> usize {
        self.server.tool_router.list_all().len()
    }

    /// Returns true if no tools are registered.
    pub fn is_empty(&self) -> bool {
        self.server.tool_router.list_all().is_empty()
    }
}

impl Default for ToolRegistry {
    /// Creates a default tool registry with an unconfigured server.
    ///
    /// Use `ToolRegistry::new()` with a configured server for production.
    fn default() -> Self {
        Self {
            server: Arc::new(crate::BotticelliServer::builder().build()),
        }
    }
}
