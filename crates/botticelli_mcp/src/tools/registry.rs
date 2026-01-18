//! Tool registry for managing and executing MCP tools.

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
    #[tracing::instrument(skip(server))]
    pub fn new(server: Arc<crate::BotticelliServer>) -> Self {
        Self { server }
    }

    /// Get tool definitions for LLM function calling.
    ///
    /// Retrieves all registered tools from the rmcp ToolRouter.
    #[instrument(skip(self))]
    pub fn tool_definitions(&self) -> Vec<botticelli_core::ToolDefinition> {
        debug!("Retrieving tool definitions from rmcp ToolRouter");
        
        let tools = self.server.get_tool_router().list_all()
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
                let result = self.server.echo(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "server_info" => {
                debug!("Delegating to server_info handler");
                let result = self.server.server_info().await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            // Narrative tools
            "create_narrative" => {
                debug!("Delegating to create_narrative handler");
                let result = self.server.create_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "modify_narrative" => {
                debug!("Delegating to modify_narrative handler");
                let result = self.server.modify_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "save_narrative" => {
                debug!("Delegating to save_narrative handler");
                let result = self.server.save_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "validate_narrative" => {
                debug!("Delegating to validate_narrative handler");
                let result = self.server.validate_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            // Execution tools
            "execute_act" => {
                debug!("Delegating to execute_act handler");
                let result = self.server.execute_act(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "execute_narrative" => {
                debug!("Delegating to execute_narrative handler");
                let result = self.server.execute_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "generate" => {
                debug!("Delegating to generate handler");
                let result = self.server.generate(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            // Elicitation session tools
            "create_narrative_session" => {
                debug!("Delegating to create_narrative_session handler");
                let result = self.server.create_narrative_session(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "elicit_metadata" => {
                debug!("Delegating to elicit_metadata handler");
                let result = self.server.elicit_metadata(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "elicit_act" => {
                debug!("Delegating to elicit_act handler");
                let result = self.server.elicit_act(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "finalize_narrative" => {
                debug!("Delegating to finalize_narrative handler");
                let result = self.server.finalize_narrative(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "elicit_carousel" => {
                debug!("Delegating to elicit_carousel handler");
                let result = self.server.elicit_carousel(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "get_narrative_state" => {
                debug!("Delegating to get_narrative_state handler");
                let result = self.server.get_narrative_state(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "validate_narrative_session" => {
                debug!("Delegating to validate_narrative_session handler");
                let result = self.server.validate_narrative_session(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "apply_validation_fixes" => {
                debug!("Delegating to apply_validation_fixes handler");
                let result = self.server.apply_validation_fixes(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            // Elicitation primitive tools
            "elicit_text" => {
                debug!("Delegating to elicit_text handler");
                let result = self.server.elicit_text(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "elicit_bool" => {
                debug!("Delegating to elicit_bool handler");
                let result = self.server.elicit_bool(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "elicit_number" => {
                debug!("Delegating to elicit_number handler");
                let result = self.server.elicit_number(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            "elicit_select" => {
                debug!("Delegating to elicit_select handler");
                let result = self.server.elicit_select(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            // Scene management tools (feature-gated)
            #[cfg(feature = "database")]
            "create_scene" => {
                debug!("Delegating to create_scene handler");
                let result = self.server.create_scene(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            #[cfg(feature = "database")]
            "list_scenes" => {
                debug!("Delegating to list_scenes handler");
                let result = self.server.list_scenes(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            #[cfg(feature = "database")]
            "update_scene" => {
                debug!("Delegating to update_scene handler");
                let result = self.server.update_scene(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            #[cfg(feature = "database")]
            "delete_scene" => {
                debug!("Delegating to delete_scene handler");
                let result = self.server.delete_scene(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            // Discord tools (feature-gated)
            #[cfg(feature = "discord")]
            "discord_post_message" => {
                debug!("Delegating to discord_post_message handler");
                let result = self.server.discord_post_message(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            #[cfg(feature = "discord")]
            "discord_get_messages" => {
                debug!("Delegating to discord_get_messages handler");
                let result = self.server.discord_get_messages(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            #[cfg(feature = "discord")]
            "discord_get_guild_info" => {
                debug!("Delegating to discord_get_guild_info handler");
                let result = self.server.discord_get_guild_info(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            #[cfg(feature = "discord")]
            "discord_get_channels" => {
                debug!("Delegating to discord_get_channels handler");
                let result = self.server.discord_get_channels(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            // Database tool (feature-gated)
            #[cfg(feature = "database")]
            "query_content" => {
                debug!("Delegating to query_content handler");
                let result = self.server.query_content(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            // Metrics tool
            "export_metrics" => {
                debug!("Delegating to export_metrics handler");
                let result = self.server.export_metrics(rmcp::handler::server::wrapper::Parameters(
                    serde_json::from_value(input)?
                ))
                .await?;
                Ok(serde_json::to_value(result.0).unwrap())
            }
            
            _ => {
                warn!(tool_name = name, "Unknown tool requested");
                Err(McpError::tool_not_found(name))
            }
        }
    }

    /// Gets the number of registered tools.
    #[tracing::instrument(skip(self))]
    pub fn len(&self) -> usize {
        self.server.get_tool_router().list_all().len()
    }

    /// Returns true if no tools are registered.
    #[tracing::instrument(skip(self))]
    pub fn is_empty(&self) -> bool {
        self.server.get_tool_router().list_all().is_empty()
    }
}

impl Default for ToolRegistry {
    /// Creates a default tool registry with an unconfigured server.
    ///
    /// Use `ToolRegistry::new()` with a configured server for production.
    #[tracing::instrument]
    fn default() -> Self {
        Self {
            server: Arc::new(
                crate::BotticelliServer::builder()
                    .build()
                    .expect("Default server should build successfully"),
            ),
        }
    }
}
