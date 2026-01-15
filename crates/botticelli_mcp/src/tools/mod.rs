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
pub use sampling::{
    LlmSampler, SamplingCoordinator, SamplingHelper,
    SamplingResult,
};
pub use sampling_session_manager::SamplingSessionManager;
// scene module is now empty - all scene tools migrated to rmcp

use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

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
#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn McpTool>>,
    metrics: Option<Arc<crate::PrometheusMetrics>>,
}

impl ToolRegistry {
    /// Creates a new tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            metrics: None,
        }
    }

    /// Creates a new tool registry with metrics collection.
    pub fn with_metrics(metrics: Arc<crate::PrometheusMetrics>) -> Self {
        Self {
            tools: HashMap::new(),
            metrics: Some(metrics),
        }
    }

    /// Get the metrics collector, if configured.
    pub fn metrics(&self) -> Option<&Arc<crate::PrometheusMetrics>> {
        self.metrics.as_ref()
    }

    /// Registers a tool.
    pub fn register(&mut self, tool: Arc<dyn McpTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Gets a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn McpTool>> {
        self.tools.get(name).cloned()
    }

    /// Lists all registered tools.
    pub fn list(&self) -> Vec<Arc<dyn McpTool>> {
        self.tools.values().cloned().collect()
    }

    /// Executes a tool by name.
    pub async fn execute(&self, name: &str, input: Value) -> McpResult<Value> {
        let tool = self
            .get(name)
            .ok_or_else(|| McpError::tool_not_found(name.to_string()))?;

        tool.execute(input).await
    }

    /// Get tool definitions for LLM function calling.
    ///
    /// Converts all registered tools into the format expected by LLMs.
    pub fn tool_definitions(&self) -> Vec<botticelli_core::ToolDefinition> {
        self.tools
            .values()
            .map(|tool| {
                botticelli_core::ToolDefinition::new(
                    tool.name().to_string(),
                    tool.description().to_string(),
                    tool.input_schema(),
                )
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // Core tools
        registry.register(Arc::new(EchoTool));
        registry.register(Arc::new(ServerInfoTool));

        // Narrative elicitation tools (LLM-driven creation)
        let narrative_registry = Arc::new(NarrativeRegistry::new());
        registry.register(Arc::new(CreateNarrativeSessionTool::new(
            narrative_registry.clone(),
        )));
        registry.register(Arc::new(ElicitMetadataTool::new(
            narrative_registry.clone(),
        )));
        registry.register(Arc::new(ElicitActTool::new(narrative_registry.clone())));
        registry.register(Arc::new(FinalizeNarrativeTool::new(
            narrative_registry.clone(),
        )));

        // Tools that need Arc-wrapped registry
        registry.register(Arc::new(ElicitCarouselTool::new(
            narrative_registry.clone(),
        )));
        registry.register(Arc::new(GetNarrativeStateTool::new(
            narrative_registry.clone(),
        )));
        registry.register(Arc::new(ValidateNarrativeSessionTool::new(
            narrative_registry.clone(),
        )));
        registry.register(Arc::new(ApplyValidationFixesTool::new(narrative_registry)));

        // Narrative generation tools migrated to rmcp (create_narrative, modify_narrative, save_narrative)

        // Scene management tools migrated to rmcp (create_scene, list_scenes, update_scene, delete_scene)

        // Execution tools (Phase 2 & 3)
        registry.register(Arc::new(GenerateTool));
        registry.register(Arc::new(ExecuteActTool::new()));
        registry.register(Arc::new(ExecuteNarrativeTool::new()));

        // Database tool (feature-gated)
        // NOTE: Database tools require explicit configuration via builder
        // They are not registered in Default implementation

        // Discord tools (feature-gated)
        #[cfg(feature = "discord")]
        {
            use crate::tools::{
                DiscordGetChannelsTool, DiscordGetGuildInfoTool, DiscordGetMessagesTool,
                DiscordPostMessageTool,
            };

            if let Ok(tool) = DiscordPostMessageTool::new() {
                registry.register(Arc::new(tool));
                tracing::info!("Discord post message tool registered");
            } else {
                tracing::warn!("Discord post message not available (check DISCORD_TOKEN)");
            }

            if let Ok(tool) = DiscordGetMessagesTool::new() {
                registry.register(Arc::new(tool));
                tracing::info!("Discord get messages tool registered");
            } else {
                tracing::warn!("Discord get messages not available (check DISCORD_TOKEN)");
            }

            if let Ok(tool) = DiscordGetGuildInfoTool::new() {
                registry.register(Arc::new(tool));
                tracing::info!("Discord get guild info tool registered");
            } else {
                tracing::warn!("Discord get guild info not available (check DISCORD_TOKEN)");
            }

            if let Ok(tool) = DiscordGetChannelsTool::new() {
                registry.register(Arc::new(tool));
                tracing::info!("Discord get channels tool registered");
            } else {
                tracing::warn!("Discord get channels not available (check DISCORD_TOKEN)");
            }
        }

        tracing::info!(
            "ToolRegistry initialized with {} tools",
            registry.tools.len()
        );
        registry
    }
}

/// Returns the number of registered tools.
impl ToolRegistry {
    /// Gets the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Returns true if no tools are registered.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}
