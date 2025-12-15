//! Unified MCP client for external tool execution via MCP servers.

use crate::tool_executor::ToolDefinition;
use crate::external_client::{ExternalMcpClient, ExternalServerConfig};
use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use botticelli_core::{Input, Message, Role};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};
use typed_builder::TypedBuilder;

/// Unified MCP client that orchestrates external tool execution.
///
/// This client connects to external MCP servers (filesystem, git, search, etc.)
/// and routes tool calls to the appropriate server.
#[derive(Debug, TypedBuilder)]
pub struct UnifiedMcpClient {
    /// External MCP server clients (by name)
    #[builder(default)]
    external_clients: HashMap<String, ExternalMcpClient>,

    /// Maximum iterations before stopping
    #[builder(default = 10)]
    max_iterations: usize,
}

/// Detailed execution result with tool call tracking.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Final response from the LLM
    pub final_response: String,
    /// Number of iterations executed
    pub iterations: usize,
    /// Tool calls made during execution (in order)
    pub tool_calls: Vec<ToolCallRecord>,
}

/// Record of a single tool call and its result.
#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    /// Name of the tool called
    pub tool_name: String,
    /// Arguments passed to the tool
    pub arguments: Value,
    /// Result from the tool execution
    pub result: String,
    /// Whether the tool execution succeeded
    pub success: bool,
}

impl UnifiedMcpClient {
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

    /// Get all available tool definitions from external servers.
    #[instrument(skip(self))]
    pub fn list_all_tools(&self) -> Vec<ToolDefinition> {
        let mut tools = Vec::new();

        // Add external tools
        for client in self.external_clients.values() {
            tools.extend(client.tools());
        }

        debug!(total_tools = tools.len(), "Listed all tools");
        tools
    }

    /// Execute a tool call by routing to appropriate server.
    #[instrument(skip(self, arguments), fields(tool_name))]
    pub async fn execute_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> McpClientResult<Value> {
        debug!("Executing tool: {}", tool_name);

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
                "Tool '{}' not found in any connected external server",
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

    /// Executes an agentic loop with detailed tracking of tool calls.
    ///
    /// This variant returns full execution details including all tool calls made.
    #[instrument(skip(self, backend, messages))]
    pub async fn execute_with_tracking<B>(
        &mut self,
        backend: &B,
        messages: Vec<Message>,
    ) -> McpClientResult<ExecutionResult>
    where
        B: LlmBackend + std::fmt::Debug,
    {
        info!("Starting unified agentic execution loop with tracking");

        let mut conversation = messages;
        let mut iterations = 0;
        let mut tool_call_records = Vec::new();

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

                // Execute tools and track results
                for call in tool_calls {
                    debug!(tool = %call.name, "Executing tool");

                    let result = self.execute_tool(&call.name, call.arguments.clone()).await;

                    let (result_str, success) = match result {
                        Ok(value) => {
                            let str_value = serde_json::to_string(&value)
                                .unwrap_or_else(|_| value.to_string());
                            (str_value, true)
                        }
                        Err(e) => (format!("Error: {}", e), false),
                    };

                    // Record the tool call
                    tool_call_records.push(ToolCallRecord {
                        tool_name: call.name.clone(),
                        arguments: call.arguments.clone(),
                        result: result_str.clone(),
                        success,
                    });

                    // Add tool result to conversation
                    conversation.push(
                        Message::builder()
                            .role(Role::User)
                            .content(vec![Input::Text(result_str)])
                            .build()
                            .expect("Valid user message"),
                    );
                }

                // Add assistant message to conversation
                conversation.push(
                    Message::builder()
                        .role(Role::Assistant)
                        .content(vec![Input::Text(response.clone())])
                        .build()
                        .expect("Valid assistant message"),
                );
            } else {
                // No tool calls - we're done
                info!(iterations, tool_calls = tool_call_records.len(), "Execution complete");
                return Ok(ExecutionResult {
                    final_response: response,
                    iterations,
                    tool_calls: tool_call_records,
                });
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
    /// Number of external servers connected
    pub external_server_count: usize,
    /// Total tools available from external servers
    pub total_tool_count: usize,
}
