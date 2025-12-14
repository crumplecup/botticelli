//! Core MCP client implementation.

use crate::tool_executor::ToolDefinition;
use crate::{McpClientError, McpClientErrorKind, McpClientResult, tool_executor::ToolExecutor};
use botticelli_core::{Input, Message, Role};
use derive_getters::Getters;
use serde_json::Value;
use tracing::{debug, info, instrument, warn};
use typed_builder::TypedBuilder;

/// MCP client that orchestrates LLM and tool interactions.
#[derive(Debug, Clone, Getters, TypedBuilder)]
pub struct McpClient {
    /// Tool executor for running MCP tools.
    #[builder(default)]
    tool_executor: Option<ToolExecutor>,

    /// Maximum iterations before stopping.
    #[builder(default = 10)]
    max_iterations: usize,
}

impl McpClient {
    /// Sets the available tools for this client.
    #[instrument(skip(self, tools))]
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        info!(tool_count = tools.len(), "Configuring tools");
        self.tool_executor = Some(ToolExecutor::new(tools));
        self
    }

    /// Executes an agentic loop with the given LLM backend.
    ///
    /// This method:
    /// 1. Sends initial messages to LLM
    /// 2. Checks for tool calls in response
    /// 3. Executes tools and feeds results back
    /// 4. Repeats until completion or max iterations
    #[instrument(skip(self, backend, messages))]
    pub async fn execute<B>(&self, backend: &B, messages: Vec<Message>) -> McpClientResult<String>
    where
        B: LlmBackend + std::fmt::Debug,
    {
        info!("Starting agentic execution loop");

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

            // Get response from LLM
            let response = backend
                .generate(&conversation)
                .await
                .map_err(|e| McpClientError::new(McpClientErrorKind::LlmError(e.to_string())))?;

            debug!("Received LLM response");

            // Check if response contains tool calls
            if let Some(tool_calls) = self.extract_tool_calls(&response) {
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

    /// Extracts tool calls from LLM response.
    fn extract_tool_calls(&self, _response: &str) -> Option<Vec<ToolCall>> {
        // TODO: Implement actual tool call parsing
        // This will need to parse structured output from LLM
        None
    }

    /// Executes multiple tool calls.
    #[instrument(skip(self, tool_calls))]
    async fn execute_tools(&self, tool_calls: Vec<ToolCall>) -> McpClientResult<Vec<String>> {
        let executor = self.tool_executor.as_ref().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::ToolExecutionFailed(
                "No tool executor configured".to_string(),
            ))
        })?;

        let mut results = Vec::new();

        for call in tool_calls {
            debug!(tool = %call.name, "Executing tool");

            let result = executor.execute(&call.name, call.arguments).await?;

            let result_str = serde_json::to_string(&result).map_err(|e| {
                McpClientError::new(McpClientErrorKind::SerializationError(e.to_string()))
            })?;

            results.push(result_str);
        }

        Ok(results)
    }
}

/// Represents a tool call from the LLM.
#[derive(Debug, Clone)]
struct ToolCall {
    name: String,
    arguments: Value,
}

/// Trait for LLM backends that can be used with the MCP client.
#[async_trait::async_trait]
pub trait LlmBackend: Send + Sync {
    /// Generates a response for the given messages.
    async fn generate(&self, messages: &[Message]) -> Result<String, Box<dyn std::error::Error>>;
}
