//! MCP client for tool execution via MCP servers (both internal and external).

use crate::approval::ApprovalManager;
use crate::external_client::{ExternalMcpClient, ExternalServerConfig};
use crate::metrics::McpClientMetrics;
use crate::retry::RetryConfig;
use crate::tool_registry::ToolRegistry;
use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use botticelli_core::{Input, Message, Role, ToolDefinition};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;
use typed_builder::TypedBuilder;

/// MCP host that orchestrates internal and external tool execution.
///
/// This host manages multiple MCP client connections:
/// - Executes internal Botticelli tools via ToolRegistry
/// - Connects to external MCP servers (filesystem, git, search, etc.)
/// - Routes tool calls to the appropriate handler
/// - Provides retry logic with exponential backoff
/// - Tracks metrics for observability
///
/// # Example
///
/// ```no_run
/// use botticelli_mcp_client::{McpHost, register_internal_tools};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create host with internal tools registered
/// let mut host = McpHost::builder().build();
/// register_internal_tools(host.internal_registry_mut(), "./narratives")?;
///
/// // Now internal narrative tools are available for LLM orchestration
/// let tools = host.list_all_tools();
/// assert!(tools.iter().any(|t| t.name == "create_narrative"));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, TypedBuilder)]
pub struct McpHost {
    /// Internal tool registry for Botticelli capabilities
    #[builder(default)]
    internal_registry: ToolRegistry,

    /// External MCP server clients (by name)
    #[builder(default)]
    external_clients: HashMap<String, ExternalMcpClient>,

    /// Approval manager for sensitive operations
    #[builder(default = ApprovalManager::auto_approve())]
    approval_manager: ApprovalManager,

    /// Retry configuration for tool execution
    #[builder(default = RetryConfig::default())]
    retry_config: RetryConfig,

    /// Optional metrics collector
    #[builder(default)]
    metrics: Option<McpClientMetrics>,

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

impl McpHost {
    /// Connects to an external MCP server and adds it to available clients.
    #[tracing::instrument(skip(self, config), fields(server = %config.name))]
    pub async fn connect_external_server(
        &mut self,
        config: ExternalServerConfig,
    ) -> McpClientResult<()> {
        let server_name = config.name.clone();
        tracing::info!("Connecting to external server: {}", server_name);

        let client =
            ExternalMcpClient::connect_with_retry(config, self.retry_config.clone()).await?;
        self.external_clients.insert(server_name.clone(), client);

        tracing::info!("External server {} connected successfully", server_name);
        Ok(())
    }

    /// Get all available tool definitions from internal registry and external servers.
    #[tracing::instrument(skip(self))]
    pub fn list_all_tools(&self) -> Vec<ToolDefinition> {
        let mut tools = Vec::new();

        // Add internal tools from registry
        tracing::debug!(
            internal_tool_count = self.internal_registry.tool_count(),
            "Fetching internal tools from registry"
        );
        for tool_info in self.internal_registry.list_tools() {
            tracing::trace!(tool_name = %tool_info.name, "Adding internal tool");
            tools.push(ToolDefinition::new(
                tool_info.name.clone(),
                tool_info.description.clone().unwrap_or_default(),
                tool_info.input_schema.clone(),
            ));
        }

        // Add external tools
        tracing::debug!(
            external_client_count = self.external_clients.len(),
            "Fetching tools from external clients"
        );
        for (server_name, client) in &self.external_clients {
            let client_tools = client.tools();
            tracing::debug!(
                server = %server_name,
                tool_count = client_tools.len(),
                tool_names = ?client_tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
                "External client tools"
            );
            tools.extend(client_tools);
        }

        tracing::info!(
            internal_tools = self.internal_registry.tool_count(),
            external_tools = self.external_clients.len(),
            total_tools = tools.len(),
            "Listed all tools"
        );
        tools
    }

    /// Execute a tool call by routing to internal registry or external server.
    ///
    /// This method includes metrics tracking and approval checks.
    #[tracing::instrument(skip(self, arguments), fields(tool_name))]
    pub async fn execute_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> McpClientResult<Value> {
        let start_time = Instant::now();
        tracing::debug!("Executing tool: {}", tool_name);

        // Check approval first
        if !self
            .approval_manager
            .request_approval(tool_name, &arguments)?
        {
            tracing::warn!("Tool call denied by approval manager: {}", tool_name);
            if let Some(metrics) = &self.metrics {
                metrics.record_tool_call(tool_name, false);
            }
            return Err(McpClientError::new(
                McpClientErrorKind::ToolExecutionFailed(format!(
                    "Tool call '{}' denied by user",
                    tool_name
                )),
            ));
        }

        // Execute the tool (retry logic is handled within tool execution)
        let result = self.execute_tool_inner(tool_name, arguments).await;

        // Record metrics
        let duration = start_time.elapsed();
        if let Some(metrics) = &self.metrics {
            metrics.record_tool_call(tool_name, result.is_ok());
            metrics.record_tool_duration(tool_name, duration.as_secs_f64());
        }

        result
    }

    /// Inner tool execution without metrics/approval.
    async fn execute_tool_inner(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> McpClientResult<Value> {
        // Try internal registry first
        if self.internal_registry.has_tool(tool_name) {
            tracing::debug!("Routing to internal tool registry");
            let content = self
                .internal_registry
                .execute_tool(tool_name, arguments)
                .await?;

            // Convert pmcp::Content to JSON Value
            let result = content
                .iter()
                .map(|c| match c {
                    pmcp::Content::Text { text } => serde_json::json!({ "text": text }),
                    pmcp::Content::Image { data, mime_type } => serde_json::json!({
                        "type": "image",
                        "data": data,
                        "mime_type": mime_type
                    }),
                    pmcp::Content::Resource {
                        uri,
                        text,
                        mime_type,
                    } => serde_json::json!({
                        "type": "resource",
                        "uri": uri,
                        "text": text,
                        "mime_type": mime_type
                    }),
                })
                .collect::<Vec<_>>();

            return Ok(serde_json::json!(result));
        }

        // Try external servers
        for (server_name, client) in &mut self.external_clients {
            if client.has_tool(tool_name) {
                tracing::debug!(server = %server_name, "Routing to external server");
                return client.call_tool(tool_name, arguments).await;
            }
        }

        // Tool not found anywhere
        Err(McpClientError::new(McpClientErrorKind::ToolNotFound(
            format!(
                "Tool '{}' not found in internal registry or any external server",
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
    #[tracing::instrument(skip(self, backend, messages))]
    pub async fn execute<B>(
        &mut self,
        backend: &B,
        messages: Vec<Message>,
    ) -> McpClientResult<String>
    where
        B: LlmBackend + std::fmt::Debug,
    {
        tracing::info!("Starting unified agentic execution loop");

        let mut conversation = messages;
        let mut iterations = 0;

        loop {
            if iterations >= self.max_iterations {
                tracing::warn!(iterations, "Maximum iterations exceeded");
                return Err(McpClientError::new(
                    McpClientErrorKind::MaxIterationsExceeded(iterations),
                ));
            }

            iterations += 1;
            tracing::debug!(iteration = iterations, "Executing iteration");

            // Get response from LLM (with tool definitions)
            let all_tools = self.list_all_tools();
            let response = backend
                .generate_with_tools(&conversation, &all_tools)
                .await
                .map_err(|e| McpClientError::new(McpClientErrorKind::LlmError(e.to_string())))?;

            tracing::debug!("Received LLM response");

            // Check if response contains tool calls
            if let Some(tool_calls) = extract_tool_calls(&response) {
                tracing::debug!(tool_call_count = tool_calls.len(), "Processing tool calls");

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
                tracing::info!(iterations, "Execution complete");
                return Ok(response);
            }
        }
    }

    /// Executes an agentic loop with detailed tracking of tool calls.
    ///
    /// This variant returns full execution details including all tool calls made.
    #[tracing::instrument(skip(self, backend, messages))]
    pub async fn execute_with_tracking<B>(
        &mut self,
        backend: &B,
        messages: Vec<Message>,
    ) -> McpClientResult<ExecutionResult>
    where
        B: LlmBackend + std::fmt::Debug,
    {
        tracing::info!("Starting unified agentic execution loop with tracking");

        let mut conversation = messages;
        let mut iterations = 0;
        let mut tool_call_records = Vec::new();

        loop {
            if iterations >= self.max_iterations {
                tracing::warn!(iterations, "Maximum iterations exceeded");
                return Err(McpClientError::new(
                    McpClientErrorKind::MaxIterationsExceeded(iterations),
                ));
            }

            iterations += 1;
            tracing::debug!(iteration = iterations, "Executing iteration");

            // Get response from LLM (with tool definitions)
            let all_tools = self.list_all_tools();
            let response = backend
                .generate_with_tools(&conversation, &all_tools)
                .await
                .map_err(|e| McpClientError::new(McpClientErrorKind::LlmError(e.to_string())))?;

            tracing::debug!("Received LLM response");

            // Check if response contains tool calls
            if let Some(tool_calls) = extract_tool_calls(&response) {
                tracing::debug!(tool_call_count = tool_calls.len(), "Processing tool calls");

                // Execute tools and track results
                for call in tool_calls {
                    tracing::debug!(tool = %call.name, "Executing tool");

                    let result = self.execute_tool(&call.name, call.arguments.clone()).await;

                    let (result_str, success) = match result {
                        Ok(value) => {
                            let str_value =
                                serde_json::to_string(&value).unwrap_or_else(|_| value.to_string());
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
                tracing::info!(
                    iterations,
                    tool_calls = tool_call_records.len(),
                    "Execution complete"
                );

                // Record workflow metrics
                if let Some(metrics) = &self.metrics {
                    metrics.record_agent_iterations(iterations, "completed");
                }

                return Ok(ExecutionResult {
                    final_response: response,
                    iterations,
                    tool_calls: tool_call_records,
                });
            }
        }
    }

    /// Executes multiple tool calls.
    #[tracing::instrument(skip(self, tool_calls))]
    async fn execute_tools(&mut self, tool_calls: Vec<ToolCall>) -> McpClientResult<Vec<String>> {
        let mut results = Vec::new();

        for call in tool_calls {
            tracing::debug!(tool = %call.name, "Executing tool");

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
            internal_tool_count: self.internal_registry.tool_count(),
            external_server_count: self.external_clients.len(),
            total_tool_count: self.list_all_tools().len(),
        }
    }

    /// Get mutable access to internal tool registry for registration.
    pub fn internal_registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.internal_registry
    }

    /// Get immutable access to internal tool registry.
    pub fn internal_registry(&self) -> &ToolRegistry {
        &self.internal_registry
    }

    /// Set metrics collector for observability.
    pub fn set_metrics(&mut self, metrics: McpClientMetrics) {
        self.metrics = Some(metrics);
    }

    /// Get reference to metrics if configured.
    pub fn metrics(&self) -> Option<&McpClientMetrics> {
        self.metrics.as_ref()
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
#[tracing::instrument(skip(response))]
pub fn extract_tool_calls(response: &str) -> Option<Vec<ToolCall>> {
    // Try to parse as JSON first (structured output)
    if let Ok(json) = serde_json::from_str::<Value>(response) {
        // Check for Anthropic tool use format
        if let Some(content) = json.get("content").and_then(|c| c.as_array()) {
            let mut calls = Vec::new();

            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                    && let (Some(name), Some(input)) =
                        (item.get("name").and_then(|n| n.as_str()), item.get("input"))
                {
                    calls.push(ToolCall {
                        name: name.to_string(),
                        arguments: input.clone(),
                    });
                }
            }

            if !calls.is_empty() {
                tracing::debug!(tool_call_count = calls.len(), "Extracted tool calls");
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
    /// Number of internal tools registered
    pub internal_tool_count: usize,
    /// Number of external servers connected
    pub external_server_count: usize,
    /// Total tools available (internal + external)
    pub total_tool_count: usize,
}

/// ChatHost implementation for McpHost.
impl botticelli_interface::ChatHost for McpHost {
    #[tracing::instrument(skip(self))]
    fn available_tools(&self) -> botticelli_error::ChatResult<Vec<ToolDefinition>> {
        Ok(self.list_all_tools())
    }

    #[tracing::instrument(skip(self, arguments), fields(tool = %name))]
    fn execute_tool(&mut self, name: &str, arguments: serde_json::Value) -> botticelli_error::ChatResult<serde_json::Value> {
        // We need to make this async-compatible, but for now use blocking
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| botticelli_error::ChatError::execution_failed(format!("Runtime error: {}", e)))?;
        
        rt.block_on(async {
            self.execute_tool_inner(name, arguments).await
                .map_err(|e| botticelli_error::ChatError::execution_failed(e.to_string()))
        })
    }
}
