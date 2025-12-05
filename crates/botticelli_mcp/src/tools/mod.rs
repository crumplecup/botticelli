//! Tool implementations for MCP server.

mod database;
#[cfg(feature = "discord")]
mod discord;
mod echo;
mod execute_act;
mod execute_narrative;
mod generate;
mod generate_llm;
mod server_info;
mod validate_narrative;

pub use database::QueryContentTool;
#[cfg(feature = "discord")]
pub use discord::{
    DiscordGetChannelsTool, DiscordGetGuildInfoTool, DiscordGetMessagesTool,
    DiscordPostMessageTool,
};
pub use echo::EchoTool;
pub use execute_act::ExecuteActTool;
pub use execute_narrative::ExecuteNarrativeTool;
pub use generate::GenerateTool;
pub use server_info::ServerInfoTool;
pub use validate_narrative::ValidateNarrativeTool;

// Export LLM tools based on features
#[cfg(feature = "gemini")]
pub use generate_llm::GenerateGeminiTool;
#[cfg(feature = "anthropic")]
pub use generate_llm::GenerateAnthropicTool;
#[cfg(feature = "ollama")]
pub use generate_llm::GenerateOllamaTool;
#[cfg(feature = "huggingface")]
pub use generate_llm::GenerateHuggingFaceTool;
#[cfg(feature = "groq")]
pub use generate_llm::GenerateGroqTool;

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
        
        // Core tools
        registry.register(Arc::new(EchoTool));
        registry.register(Arc::new(ServerInfoTool));
        
        // Validation tool
        registry.register(Arc::new(ValidateNarrativeTool));
        
        // Execution tools (Phase 2 & 3)
        registry.register(Arc::new(GenerateTool));
        registry.register(Arc::new(ExecuteActTool::new()));
        registry.register(Arc::new(ExecuteNarrativeTool::new()));
        
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
        }
        
        tracing::info!("ToolRegistry initialized with {} tools", registry.tools.len());
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
