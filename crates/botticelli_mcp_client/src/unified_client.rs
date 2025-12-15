//! Unified MCP client that combines internal and external tool execution.

use crate::tool_executor::{ToolDefinition, ToolExecutor};
use crate::external_client::{ExternalMcpClient, ExternalServerConfig};
use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use botticelli_core::{Input, Message, Role};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};
use typed_builder::TypedBuilder;

/// Unified MCP client that orchestrates internal and external tool execution.
#[derive(Debug, TypedBuilder)]
pub struct UnifiedMcpClient {
    /// Internal tool executor for botticelli-native tools
    #[builder(default)]
    internal_executor: Option<ToolExecutor>,

    /// External MCP server clients (by name)
    #[builder(default)]
    external_clients: HashMap<String, ExternalMcpClient>,

    /// Maximum iterations before stopping
    #[builder(default = 10)]
    max_iterations: usize,

    /// Whether to automatically route tool calls to appropriate executor
    #[builder(default = true)]
    auto_routing: bool,
}

impl UnifiedMcpClient {
    /// Sets internal tools for this client.
    #[instrument(skip(self, tools))]
    pub fn with_internal_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        info!(tool_count = tools.len(), "Configuring internal tools");
        self.internal_executor = Some(ToolExecutor::new(tools));
        self
    }

    /// Connects to an external MCP server and adds it to available clients.
    #[instrument(skip(self, config), fields(server = %config.name))]
    pub async fn connect_external_server(
        &mut self,
        config: ExternalServerConfig,
    ) -> McpClientResult<()> {
        let server_name = config.name.clone();
        info!("Connecting to external server: {}", server_name);

        let client = ExternalMcpClient::connect(config).await?;
        self.external_clients.insert(server_name.clone(), client);

        info!("External server {} connected successfully", server_name);
        Ok(())
    }

    /// Get all available tool definitions (internal + external).
    #[instrument(skip(self))]
    pub fn list_all_tools(&self) -> Vec<ToolDefinition> {
        let mut tools = Vec::new();

        // Add internal tools
        if let Some(executor) = &self.internal_executor {
            tools.extend(executor.available_tools().iter().map(|t| (*t).clone()));
        }

        // Add external tools
        for client in self.external_clients.values() {
            tools.extend(client.tools());
        }

        debug!(total_tools = tools.len(), "Listed all tools");
        tools
    }

    /// Execute a tool call by routing to appropriate executor.
    #[instrument(skip(self, arguments), fields(tool_name))]
    pub async fn execute_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> McpClientResult<Value> {
        debug!("Executing tool: {}", tool_name);

        // Try internal tools first
        if let Some(executor) = &self.internal_executor {
            if executor
                .available_tools()
                .iter()
                .any(|t| t.name == tool_name)
            {
                debug!("Routing to internal executor");
                return executor.execute(tool_name, arguments).await;
            }
        }

        // Try external servers
        for (server_name, client) in &mut self.external_clients {
            if client.has_tool(tool_name) {
                debug!(server = %server_name, "Routing to external server");
                return client.call_tool(tool_name, arguments).await;
            }
        }

        // Tool not found anywhere
        Err(McpClientError::new(McpClientErrorKind::ToolNotFound(
            format!(
                "Tool '{}' not found in internal executor or any external server",
                tool_name
            ),
        )))
    }

    /// Executes an agentic loop with the given LLM backend.
    ///
    /// This method:
    /// 1. Sends initial messages to LLM
    /// 2. Checks for tool calls in response
    /// 3. Executes tools and feeds results back
    /// 4. Repeats until completion or max iterations
    #[instrument(skip(self, backend, messages))]
    pub async fn execute<B>(&mut self, backend: &B, messages: Vec<Message>) -> McpClientResult<String>
    where
        B: LlmBackend + std::fmt::Debug,
    {
        info!("Starting unified agentic execution loop");

        let mut conversation = messages;
        let mut iterations = 0;

        loop {
            if iterations >= self.max_iterations {
                warn!(iterations, "Maximum iterations exceeded");
                return Err(McpClientError::new(
                    McpClientErrorKind::MaxIterationsExceeded(iterations),
                ));
            }

            iterations += 1;
            debug!(iteration = iterations, "Executing iteration");

            // Get response from LLM (with tool definitions)
            let all_tools = self.list_all_tools();
            let response = backend
                .generate_with_tools(&conversation, &all_tools)
                .await
                .map_err(|e| McpClientError::new(McpClientErrorKind::LlmError(e.to_string())))?;

            debug!("Received LLM response");

            // Check if response contains tool calls
            if let Some(tool_calls) = extract_tool_calls(&response) {
                debug!(tool_call_count = tool_calls.len(), "Processing tool calls");

                // Execute tools
                let tool_results = self.execute_tools(tool_calls).await?;

                // Add assistant message and tool results to conversation
                conversation.push(
                    Message::builder()
                        .role(Role::Assistant)
                        .content(vec![Input::Text(response.clone())])
                        .build()
                        .expect("Valid assistant message"),
                );

                for result in tool_results {
                    conversation.push(
                        Message::builder()
                            .role(Role::User)
                            .content(vec![Input::Text(result)])
                            .build()
                            .expect("Valid user message"),
                    );
                }
            } else {
                // No tool calls - we're done
                info!(iterations, "Execution complete");
                return Ok(response);
            }
        }
    }

    /// Executes multiple tool calls.
    #[instrument(skip(self, tool_calls))]
    async fn execute_tools(&mut self, tool_calls: Vec<ToolCall>) -> McpClientResult<Vec<String>> {
        let mut results = Vec::new();

        for call in tool_calls {
            debug!(tool = %call.name, "Executing tool");

            let result = self.execute_tool(&call.name, call.arguments).await?;

            let result_str = serde_json::to_string(&result).map_err(|e| {
                McpClientError::new(McpClientErrorKind::SerializationError(e.to_string()))
            })?;

            results.push(result_str);
        }

        Ok(results)
    }

    /// Get metrics about connected servers and tool usage.
    pub fn get_metrics(&self) -> UnifiedClientMetrics {
        UnifiedClientMetrics {
            internal_tool_count: self
                .internal_executor
                .as_ref()
                .map(|e| e.available_tools().len())
                .unwrap_or(0),
            external_server_count: self.external_clients.len(),
            total_tool_count: self.list_all_tools().len(),
        }
    }
}

/// Represents a tool call from the LLM.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Name of the tool to execute.
    pub name: String,
    /// Arguments to pass to the tool.
    pub arguments: Value,
}

/// Extracts tool calls from LLM response.
///
/// This parses structured output from the LLM that indicates tool usage.
/// Currently supports Anthropic's tool use format.
#[instrument(skip(response))]
pub fn extract_tool_calls(response: &str) -> Option<Vec<ToolCall>> {
    // Try to parse as JSON first (structured output)
    if let Ok(json) = serde_json::from_str::<Value>(response) {
        // Check for Anthropic tool use format
        if let Some(content) = json.get("content").and_then(|c| c.as_array()) {
            let mut calls = Vec::new();

            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if let (Some(name), Some(input)) = (
                        item.get("name").and_then(|n| n.as_str()),
                        item.get("input"),
                    ) {
                        calls.push(ToolCall {
                            name: name.to_string(),
                            arguments: input.clone(),
                        });
                    }
                }
            }

            if !calls.is_empty() {
                debug!(tool_call_count = calls.len(), "Extracted tool calls");
                return Some(calls);
            }
        }
    }

    // No tool calls found
    None
}

/// Trait for LLM backends that can be used with the unified MCP client.
#[async_trait::async_trait]
pub trait LlmBackend: Send + Sync {
    /// Generates a response for the given messages with tool definitions.
    async fn generate_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<String, Box<dyn std::error::Error>>;
}

/// Metrics about the unified client state.
#[derive(Debug, Clone)]
pub struct UnifiedClientMetrics {
    /// Number of internal tools
    pub internal_tool_count: usize,
    /// Number of external servers connected
    pub external_server_count: usize,
    /// Total tools available (internal + external)
    pub total_tool_count: usize,
}
