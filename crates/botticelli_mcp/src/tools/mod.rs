//! Tool implementations and shared utilities for the MCP server.

#[cfg(feature = "discord")]
pub(crate) mod discord;
#[cfg(feature = "discord")]
mod discord_workflow;
pub(crate) mod generate_llm;
mod metrics;
pub(crate) mod modify_narrative;
mod narrative_processor;
pub(crate) mod narrative_utils;
pub(crate) mod narrative_validation_helpers;
mod prometheus;
mod sampling;

pub use metrics::{ActMetrics, ExecutionMetrics};
pub use narrative_utils::{Act, NarrativeHelper};
pub use prometheus::{MetricsSummary, PrometheusMetrics};
pub use sampling::{
    LlmSampler, SamplingCoordinator, SamplingError, SamplingErrorKind, SamplingHelper,
    SamplingResult,
};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
pub use narrative_processor::McpProcessorCollector;

use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for MCP tools.
///
/// Retained for backward compatibility with `SamplingCoordinator` and external crates.
/// New tools should be implemented as `#[tool]` methods on `BotticelliServer` instead.
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
/// Retained for backward compatibility with `SamplingCoordinator` and external crates.
/// `BotticelliServer` now uses `#[tool]` methods and `DynamicToolRegistry` directly.
#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn McpTool>>,
}

impl ToolRegistry {
    /// Creates a new tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
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

    /// Gets the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Returns true if no tools are registered.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
