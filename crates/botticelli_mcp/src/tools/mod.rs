//! Tool implementations for MCP server.

mod bot_commands;
mod create_narrative;
#[cfg(feature = "discord")]
mod discord;
mod echo;
mod elicitation;
mod elicitation_primitives;
mod elicitation_tools;
mod execute_act;
mod execute_narrative;
mod generate;
mod get_narrative_state;
mod metrics;
mod modify_narrative;
mod narrative;
mod narrative_creation;
mod narrative_processor;
pub mod narrative_utils;
pub mod narrative_validation_helpers;
mod prometheus;
mod sampling;
mod sampling_session_manager;
mod save_narrative;
mod scene;
mod server_info;
mod validate_narrative_session;

pub use bot_commands::{BotCommandRequest, BotCommandResponse};
// CreateNarrativeTool migrated to rmcp (create_narrative method in rmcp_server)
#[cfg(feature = "discord")]
pub use discord::{
    DiscordGetChannelsTool, DiscordGetGuildInfoTool, DiscordGetMessagesTool, DiscordPostMessageTool,
};
pub use echo::EchoTool;
pub use elicitation::{
    ApplyValidationFixesInput, ApplyValidationFixesOutput, CreateNarrativeSessionTool,
    ElicitActTool, ElicitCarouselTool, ElicitMetadataTool, ElicitationHelper,
    FinalizeNarrativeTool, GetNarrativeStateInput, GetNarrativeStateOutput, NarrativeRegistry,
    PartialNarrativeRegistry, ValidateNarrativeInput, ValidateNarrativeOutput,
};
// elicitation_primitives module is now empty - all primitives migrated to rmcp
pub use execute_act::ExecuteActTool;
pub use execute_narrative::ExecuteNarrativeTool;
pub use generate::GenerateTool;
pub use get_narrative_state::GetNarrativeStateTool;
pub use metrics::{ActMetrics, ExecutionMetrics};
// ModifyNarrativeTool migrated to rmcp (modify_narrative method in rmcp_server)
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
// SaveNarrativeTool migrated to rmcp (save_narrative method in rmcp_server)
pub use server_info::ServerInfoTool;
pub use validate_narrative_session::{ApplyValidationFixesTool, ValidateNarrativeSessionTool};

// Export shared narrative utilities
pub use narrative_utils::{Act, NarrativeHelper};
pub use sampling::{SamplingCoordinator, SamplingHelper, SamplingResult};
pub use sampling_session_manager::SamplingSessionManager;

// Re-export LlmSamplerOperations for convenience
pub use botticelli_interface::LlmSamplerOperations;

// scene module is now empty - all scene tools migrated to rmcp

use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, instrument, trace, warn};

/// Trait for MCP tools.
#[async_trait]
pub trait McpTool: Send + Sync {
    /// Returns the tool name.
    fn name(&self) -> &str;

    /// Returns the tool description for the LLM.
    fn description(&self) -> &str;

    /// Returns the input schema as JSON Schema.
    fn input_schema(&self) -> Value;

    /// Executes the tool with the given input.
    async fn execute(&self, input: Value) -> McpResult<Value>;
}

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
