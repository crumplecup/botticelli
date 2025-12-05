//! Tool implementations for MCP server.

mod echo;
mod server_info;

pub use echo::EchoTool;
pub use server_info::ServerInfoTool;

use crate::{McpError, McpResult};
use async_trait::async_trait;
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
            .ok_or_else(|| McpError::ToolNotFound(name.to_string()))?;

        tool.execute(input).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(EchoTool));
        registry.register(Arc::new(ServerInfoTool));
        registry
    }
}
