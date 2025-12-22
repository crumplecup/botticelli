//! Multi-turn conversation loop with tool calling support.

use botticelli_core::{
    GenerateRequest, Input, Message, MessageBuilder, Output, Role, ToolCall, ToolDefinition,
};
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use botticelli_interface::ToolCalling;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument, warn};

use crate::ToolCallHandler;

/// Maximum turns in a conversation loop to prevent infinite loops.
const MAX_CONVERSATION_TURNS: usize = 10;

/// Orchestrates multi-turn conversations with tool calling support.
pub struct ConversationLoop {
    tool_handler: Arc<Mutex<ToolCallHandler>>,
}

impl ConversationLoop {
    /// Create a new conversation loop.
    #[instrument(skip(tool_handler))]
    pub fn new(tool_handler: Arc<Mutex<ToolCallHandler>>) -> Self {
        Self { tool_handler }
    }

    /// Run a complete conversation turn, executing tools until completion.
    ///
    /// # Arguments
    /// * `provider` - LLM provider that supports tool calling
    /// * `messages` - Current conversation history
    /// * `available_tools` - Tools the LLM can call
    ///
    /// # Returns
    /// Updated message history including assistant responses and tool results
    #[instrument(skip(self, provider, messages, available_tools))]
    pub async fn run_conversation<P>(
        &self,
        provider: &P,
        mut messages: Vec<Message>,
        available_tools: &[ToolDefinition],
    ) -> ChatResult<Vec<Message>>
    where
        P: ToolCalling + Send + Sync + ?Sized,
    {
        info!(
            message_count = messages.len(),
            tool_count = available_tools.len(),
            tool_names = ?available_tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
            "Starting conversation loop"
        );

        let mut turn_count = 0;

        loop {
            turn_count += 1;

            if turn_count > MAX_CONVERSATION_TURNS {
                warn!(
                    turn_count,
                    "Reached maximum conversation turns, stopping loop"
                );
                return Err(ChatError::new(ChatErrorKind::ExecutionFailed(
                    format!("Conversation exceeded maximum turns ({})", MAX_CONVERSATION_TURNS),
                )));
            }

            debug!(turn_count, "Starting conversation turn");

            // Build request from current message history
            let request = GenerateRequest::builder()
                .messages(messages.clone())
                .build()
                .map_err(|e| {
                    ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                        "Failed to build request: {}",
                        e
                    )))
                })?;

            // Generate LLM response with tool support
            info!(
                turn = turn_count,
                tool_count = available_tools.len(),
                tool_names = ?available_tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
                "Calling LLM with tools"
            );
            let response = provider
                .generate_with_tools(&request, available_tools)
                .await
                .map_err(|e| {
                    warn!(error = %e, "LLM generation failed");
                    ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                        "LLM generation failed: {}",
                        e
                    )))
                })?;
            
            info!("Received response from LLM");

            // Extract output from response
            let output = response
                .outputs()
                .first()
                .ok_or_else(|| {
                    ChatError::new(ChatErrorKind::ValidationError(
                        "No output in LLM response".to_string(),
                    ))
                })?
                .clone();

            // Convert output to message content
            let content = match &output {
                Output::Text(text) => vec![Input::Text(text.clone())],
                Output::ToolCalls(calls) => calls
                    .iter()
                    .map(|call| Input::ToolCall {
                        id: call.id().clone(),
                        name: call.name().clone(),
                        arguments: call.arguments().clone(),
                    })
                    .collect(),
                // Handle other output types as text for now
                _ => vec![Input::Text(format!("{:?}", output))],
            };

            // Create assistant message
            let assistant_message = MessageBuilder::default()
                .role(Role::Assistant)
                .content(content.clone())
                .build()
                .map_err(|e| {
                    ChatError::new(ChatErrorKind::ValidationError(format!(
                        "Failed to build assistant message: {}",
                        e
                    )))
                })?;

            // Add assistant response to history
            messages.push(assistant_message);

            // Extract tool calls for execution
            let tool_calls: Vec<ToolCall> = if let Output::ToolCalls(calls) = output {
                calls
            } else {
                Vec::new()
            };

            // If no tool calls, conversation is complete
            if tool_calls.is_empty() {
                info!(turn_count, "Conversation complete (no tool calls)");
                return Ok(messages);
            }

            info!(
                tool_call_count = tool_calls.len(),
                turn_count,
                "Executing tool calls"
            );

            // Execute tool calls
            let handler = self.tool_handler.lock().await;
            let tool_results = handler.execute_tool_calls(tool_calls).await?;

            // Add tool results to message history
            for result in tool_results {
                let result_message = MessageBuilder::default()
                    .role(Role::User)
                    .content(vec![Input::ToolResult {
                        tool_call_id: result.tool_call_id().clone(),
                        content: result.content().to_string(),
                        is_error: *result.is_error(),
                    }])
                    .build()
                    .map_err(|e| {
                        ChatError::new(ChatErrorKind::ValidationError(format!(
                            "Failed to build tool result message: {}",
                            e
                        )))
                    })?;

                messages.push(result_message);
            }

            debug!(
                turn_count,
                message_count = messages.len(),
                "Turn complete, continuing loop"
            );
        }
    }
}
