use crate::llm_adapter::{
    FinishReason, GenerationConfig, LlmAdapter, Message, MessageRole, ToolCall as LlmToolCall,
    ToolSchema as LlmToolSchema,
};
use crate::schema::{ToolSchema, ToolSchemaConverter};
use crate::tool_registry::ToolRegistry;
use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use botticelli_core::ToolResult;
use pmcp::{Content, ToolInfo};
use serde_json::Value;
use std::sync::Arc;

/// Orchestrates LLM interactions with tool execution.
///
/// This is the main entry point for self-driving Botticelli operations.
/// It manages the agentic loop: LLM → Tool Calls → Tool Execution → LLM.
#[derive(Clone, derive_getters::Getters)]
pub struct Orchestrator {
    /// Tool registry for executing tools
    registry: Arc<ToolRegistry>,
    /// LLM adapter for generation
    adapter: Arc<dyn LlmAdapter>,
    /// Maximum iterations before stopping
    max_iterations: usize,
}

impl Orchestrator {
    /// Create a new orchestrator.
    #[tracing::instrument(skip(registry, adapter))]
    pub fn new(
        registry: Arc<ToolRegistry>,
        adapter: Arc<dyn LlmAdapter>,
        max_iterations: usize,
    ) -> Self {
        tracing::info!(
            max_iterations,
            model = adapter.model_name(),
            "Creating orchestrator"
        );
        Self {
            registry,
            adapter,
            max_iterations,
        }
    }

    /// Execute an agentic loop with the given initial messages.
    ///
    /// This method:
    /// 1. Sends messages to LLM with tool definitions
    /// 2. Checks for tool calls in response
    /// 3. Executes tools via registry
    /// 4. Feeds results back to LLM
    /// 5. Repeats until completion or max iterations
    #[tracing::instrument(skip(self, messages))]
    pub async fn execute(&self, messages: Vec<Message>) -> McpClientResult<String> {
        tracing::info!("Starting agentic execution loop");

        let mut conversation = messages;
        let mut iteration = 0;

        loop {
            if iteration >= self.max_iterations {
                tracing::warn!(iteration, "Maximum iterations exceeded");
                return Err(McpClientError::new(
                    McpClientErrorKind::MaxIterationsExceeded(iteration),
                ));
            }

            iteration += 1;
            tracing::debug!(iteration, "Executing iteration");

            // Convert tool registry to LLM schemas
            let tool_schemas = self.get_tool_schemas();

            // Generate response from LLM
            let config = GenerationConfig::default();
            let response = self
                .adapter
                .generate(conversation.clone(), tool_schemas, config)
                .await?;

            tracing::debug!(
                finish_reason = ?response.finish_reason(),
                tokens = response.usage().total_tokens(),
                "Received LLM response"
            );

            // Check finish reason
            match response.finish_reason() {
                FinishReason::Stop => {
                    // Natural completion - return final message
                    tracing::info!(iteration, "Execution complete");
                    return Ok(response.message().content().clone());
                }
                FinishReason::ToolCalls => {
                    // Process tool calls
                    tracing::debug!(
                        tool_call_count = response.message().tool_calls().len(),
                        "Processing tool calls"
                    );

                    let tool_results = self.execute_tools(response.message().tool_calls()).await?;

                    // Add assistant message and tool results to conversation
                    conversation.push(response.message().clone());

                    let tool_message =
                        Message::new(MessageRole::Tool, String::new(), Vec::new(), tool_results);
                    conversation.push(tool_message);
                }
                FinishReason::MaxTokens => {
                    tracing::warn!("Hit max tokens limit");
                    return Err(McpClientError::new(McpClientErrorKind::LlmError(
                        "Hit max tokens limit".to_string(),
                    )));
                }
                FinishReason::ContentFilter => {
                    tracing::warn!("Content filtered");
                    return Err(McpClientError::new(McpClientErrorKind::LlmError(
                        "Content filtered".to_string(),
                    )));
                }
                FinishReason::Error => {
                    tracing::warn!("LLM error");
                    return Err(McpClientError::new(McpClientErrorKind::LlmError(
                        "LLM returned error finish reason".to_string(),
                    )));
                }
            }
        }
    }

    /// Get tool schemas for LLM context.
    #[tracing::instrument(skip(self))]
    fn get_tool_schemas(&self) -> Vec<LlmToolSchema> {
        let tool_infos = self.registry.list_tools();
        tracing::debug!(tool_count = tool_infos.len(), "Converting tool schemas");

        tool_infos
            .into_iter()
            .map(|info| {
                LlmToolSchema::new(
                    info.name.clone(),
                    info.description.clone().unwrap_or_default(),
                    info.input_schema.clone(),
                )
            })
            .collect()
    }

    /// Execute multiple tool calls.
    #[tracing::instrument(skip(self, tool_calls))]
    async fn execute_tools(&self, tool_calls: &[LlmToolCall]) -> McpClientResult<Vec<ToolResult>> {
        let mut results = Vec::new();

        for call in tool_calls {
            tracing::debug!(tool = %call.name(), id = %call.id(), "Executing tool");

            let result = self
                .registry
                .execute_tool(call.name(), call.arguments().clone())
                .await;

            let tool_result = match result {
                Ok(content) => {
                    let content_json = content_to_json(&content)?;
                    ToolResult::new(call.id().clone(), content_json, false)
                }
                Err(e) => {
                    tracing::warn!(tool = %call.name(), error = ?e, "Tool execution failed");
                    ToolResult::new(
                        call.id().clone(),
                        serde_json::json!({
                            "error": e.to_string()
                        }),
                        true,
                    )
                }
            };

            results.push(tool_result);
        }

        Ok(results)
    }
}

/// Convert pmcp Content to JSON for LLM consumption.
#[tracing::instrument(skip(content))]
fn content_to_json(content: &[Content]) -> McpClientResult<Value> {
    let mut items = Vec::new();

    for item in content {
        let json = match item {
            Content::Text { text } => serde_json::json!({
                "type": "text",
                "text": text
            }),
            Content::Image { data, mime_type } => serde_json::json!({
                "type": "image",
                "data": data,
                "mimeType": mime_type
            }),
            Content::Resource { uri, text, .. } => serde_json::json!({
                "type": "resource",
                "uri": uri,
                "text": text
            }),
        };
        items.push(json);
    }

    Ok(serde_json::json!({ "content": items }))
}

/// Convert ToolInfo to generic ToolSchema.
pub fn tool_info_to_schema(info: &ToolInfo) -> ToolSchema {
    ToolSchema::new(
        info.name.clone(),
        info.description.clone().unwrap_or_default(),
        info.input_schema.clone(),
    )
}

/// Convert ToolInfo to provider-specific schema.
pub fn tool_info_to_provider_schema<T: ToolSchemaConverter>(info: &ToolInfo) -> T::Output {
    let schema = tool_info_to_schema(info);
    T::convert(&schema)
}
