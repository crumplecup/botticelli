//! Tool implementations for MCP server.

mod bot_commands;
mod create_narrative;
mod database;
#[cfg(feature = "discord")]
mod discord;
#[cfg(feature = "discord")]
mod discord_workflow;
mod echo;
mod elicitation;
mod execute_act;
mod execute_narrative;
mod export_metrics;
mod generate;
mod generate_llm;
mod metrics;
mod modify_narrative;
mod narrative_creation;
mod narrative_processor;
mod narrative_utils;
mod narrative_validation_helpers;
mod prometheus;
mod sampling;
mod sampling_session_manager;
mod save_narrative;
mod server_info;
#[cfg(feature = "discord")]
mod social;
mod validate_narrative;

pub use bot_commands::{BotCommandRequest, BotCommandResponse};
pub use create_narrative::CreateNarrativeTool;
pub use database::QueryContentTool;
#[cfg(feature = "discord")]
pub use discord::{
    DiscordGetChannelsTool, DiscordGetGuildInfoTool, DiscordGetMessagesTool, DiscordPostMessageTool,
};
#[cfg(feature = "discord")]
pub use discord_workflow::DiscordContentWorkflowTool;
pub use echo::EchoTool;
pub use elicitation::{
    CreateNarrativeSessionTool, ElicitActTool, ElicitMetadataTool, ElicitationHelper,
    FinalizeNarrativeTool, NarrativeRegistry,
};
pub use execute_act::ExecuteActTool;
pub use execute_narrative::ExecuteNarrativeTool;
pub use export_metrics::ExportMetricsTool;
pub use generate::GenerateTool;
pub use metrics::{ActMetrics, ExecutionMetrics};
pub use modify_narrative::ModifyNarrativeTool;
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
pub use save_narrative::SaveNarrativeTool;
pub use server_info::ServerInfoTool;
#[cfg(feature = "discord")]
pub use social::{DiscordBotCommandTool, DiscordPostTool};
pub use validate_narrative::ValidateNarrativeTool;

// Export shared narrative utilities
pub use narrative_utils::{Act, NarrativeHelper};
pub use sampling::{
    LlmSampler, SamplingCoordinator, SamplingError, SamplingErrorKind, SamplingHelper,
    SamplingResult, ToolDefinition,
};
pub use sampling_session_manager::SamplingSessionManager;

// Export LLM tools based on features
#[cfg(feature = "anthropic")]
pub use generate_llm::GenerateAnthropicTool;
#[cfg(feature = "gemini")]
pub use generate_llm::GenerateGeminiTool;
#[cfg(feature = "groq")]
pub use generate_llm::GenerateGroqTool;
#[cfg(feature = "huggingface")]
pub use generate_llm::GenerateHuggingFaceTool;
#[cfg(feature = "ollama")]
pub use generate_llm::GenerateOllamaTool;

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
    pub fn tool_definitions(&self) -> Vec<crate::ToolDefinition> {
        self.tools
            .values()
            .map(|tool| crate::ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.input_schema(),
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

        // Validation tool
        registry.register(Arc::new(ValidateNarrativeTool));

        // Narrative elicitation tools (LLM-driven creation)
        let narrative_registry = NarrativeRegistry::new();
        registry.register(Arc::new(CreateNarrativeSessionTool::new(
            narrative_registry.clone(),
        )));
        registry.register(Arc::new(ElicitMetadataTool::new(narrative_registry.clone())));
        registry.register(Arc::new(ElicitActTool::new(narrative_registry.clone())));
        registry.register(Arc::new(FinalizeNarrativeTool::new(narrative_registry)));

        // Narrative generation tools (Phase 1)
        registry.register(Arc::new(CreateNarrativeTool));
        registry.register(Arc::new(ModifyNarrativeTool));
        registry.register(Arc::new(SaveNarrativeTool));

        // Execution tools (Phase 2 & 3)
        registry.register(Arc::new(GenerateTool));
        registry.register(Arc::new(ExecuteActTool::new()));
        registry.register(Arc::new(ExecuteNarrativeTool::new()));

        // Metrics tool
        if let Some(ref metrics) = registry.metrics {
            registry.register(Arc::new(ExportMetricsTool::new(Arc::clone(metrics))));
        }

        // Execution tools (Phase 4 - Multi-backend LLM integration)
        #[cfg(feature = "gemini")]
        if let Ok(tool) = GenerateGeminiTool::new() {
            registry.register(Arc::new(tool));
            tracing::info!("Gemini generation tool registered");
        } else {
            tracing::warn!("Gemini not available (check GEMINI_API_KEY)");
        }

        #[cfg(feature = "anthropic")]
        if let Ok(tool) = GenerateAnthropicTool::new() {
            registry.register(Arc::new(tool));
            tracing::info!("Anthropic generation tool registered");
        } else {
            tracing::warn!("Anthropic not available (check ANTHROPIC_API_KEY)");
        }

        #[cfg(feature = "ollama")]
        if let Ok(tool) = GenerateOllamaTool::new() {
            registry.register(Arc::new(tool));
            tracing::info!("Ollama generation tool registered");
        } else {
            tracing::warn!("Ollama not available (check OLLAMA_HOST)");
        }

        #[cfg(feature = "huggingface")]
        if let Ok(tool) = GenerateHuggingFaceTool::new() {
            registry.register(Arc::new(tool));
            tracing::info!("HuggingFace generation tool registered");
        } else {
            tracing::warn!("HuggingFace not available (check HUGGINGFACE_API_KEY)");
        }

        #[cfg(feature = "groq")]
        if let Ok(tool) = GenerateGroqTool::new() {
            registry.register(Arc::new(tool));
            tracing::info!("Groq generation tool registered");
        } else {
            tracing::warn!("Groq not available (check GROQ_API_KEY)");
        }

        // Database tool (feature-gated)
        #[cfg(feature = "database")]
        registry.register(Arc::new(QueryContentTool));

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

            // Social media integration tools (Phase 5)
            if let Ok(discord_token) = std::env::var("DISCORD_TOKEN") {
                if let Ok(tool) = DiscordBotCommandTool::new(discord_token.clone()) {
                    registry.register(Arc::new(tool));
                    tracing::info!("Discord bot command tool registered");
                }

                if let Ok(tool) = DiscordPostTool::new(discord_token) {
                    registry.register(Arc::new(tool));
                    tracing::info!("Discord post tool registered");
                }
            } else {
                tracing::warn!("Discord bot tools not available (check DISCORD_TOKEN)");
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
