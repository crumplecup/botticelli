use botticelli_core::{GenerateRequest, GenerateResponse, Input, Message, Role};
use botticelli_interface::BotticelliDriver;
use botticelli_mcp::{
    ConversationSession, ConversationTurn, LlmSampler, SamplingError, SamplingErrorKind,
    ToolDefinition, ToolRegistry, ToolResult,
};
use std::sync::Arc;
use tracing::{debug, error, instrument};

/// LLM sampler implementation for chat system.
pub struct ChatLlmSampler {
    /// LLM provider (Anthropic, OpenAI, etc.)
    provider: Arc<dyn BotticelliDriver>,

    /// Tool registry for execution
    tool_registry: Arc<ToolRegistry>,
}

impl ChatLlmSampler {
    /// Create new sampler with provider and tool registry.
    pub fn new(provider: Arc<dyn BotticelliDriver>, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            provider,
            tool_registry,
        }
    }

    /// Build GenerateRequest from conversation session.
    fn build_request(
        &self,
        session: &ConversationSession,
    ) -> Result<GenerateRequest, SamplingError> {
        let mut messages = Vec::new();

        // Add all conversation turns as messages
        for turn in &session.turns {
            match turn {
                ConversationTurn::UserMessage { content, .. } => {
                    messages.push(
                        Message::builder()
                            .role(Role::User)
                            .content(vec![Input::Text(content.clone())])
                            .build()
                            .map_err(|e| {
                                SamplingError::new(SamplingErrorKind::RequestBuildingFailed(
                                    e.to_string(),
                                ))
                            })?,
                    );
                }

                ConversationTurn::AssistantMessage { content } => {
                    messages.push(
                        Message::builder()
                            .role(Role::Assistant)
                            .content(vec![Input::Text(content.clone())])
                            .build()
                            .map_err(|e| {
                                SamplingError::new(SamplingErrorKind::RequestBuildingFailed(
                                    e.to_string(),
                                ))
                            })?,
                    );
                }

                ConversationTurn::AssistantToolCalls { calls, thinking } => {
                    // Add thinking text if present, then tool calls
                    let mut content = Vec::new();
                    if let Some(text) = thinking {
                        content.push(Input::Text(text.clone()));
                    }

                    // Add tool call inputs
                    for call in calls {
                        content.push(Input::ToolCall {
                            id: call.id().clone(),
                            name: call.name().clone(),
                            arguments: call.arguments().clone(),
                        });
                    }

                    messages.push(
                        Message::builder()
                            .role(Role::Assistant)
                            .content(content)
                            .build()
                            .map_err(|e| {
                                SamplingError::new(SamplingErrorKind::RequestBuildingFailed(
                                    e.to_string(),
                                ))
                            })?,
                    );
                }

                ConversationTurn::ToolResults { results } => {
                    // Tool results go back as user messages
                    let content: Vec<_> = results
                        .iter()
                        .map(|result| Input::ToolResult {
                            tool_call_id: result.tool_call_id.clone(),
                            content: result.output.to_string(),
                            is_error: result.is_error,
                        })
                        .collect();

                    messages.push(
                        Message::builder()
                            .role(Role::User)
                            .content(content)
                            .build()
                            .map_err(|e| {
                                SamplingError::new(SamplingErrorKind::RequestBuildingFailed(
                                    e.to_string(),
                                ))
                            })?,
                    );
                }
            }
        }

        // Build final request
        GenerateRequest::builder()
            .messages(messages)
            .build()
            .map_err(|e| {
                SamplingError::new(SamplingErrorKind::RequestBuildingFailed(e.to_string()))
            })
    }
}

#[async_trait::async_trait]
impl LlmSampler for ChatLlmSampler {
    #[instrument(skip(self, session, _available_tools))]
    async fn generate(
        &self,
        session: &ConversationSession,
        _available_tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, SamplingError> {
        debug!(
            turn_count = session.turn_count(),
            "Generating next response"
        );

        // Build request from session
        let request = self.build_request(session)?;

        // TODO: Add tools to request once GenerateRequest supports it
        // request = request.with_tools(available_tools);

        // Call provider
        let response = self
            .provider
            .generate(&request)
            .await
            .map_err(|e| SamplingError::new(SamplingErrorKind::ProviderError(e.to_string())))?;

        debug!(output_count = response.outputs().len(), "Received response");

        Ok(response)
    }

    #[instrument(skip(self, calls))]
    async fn execute_tools(
        &self,
        calls: &[botticelli_core::ToolCall],
    ) -> Result<Vec<ToolResult>, SamplingError> {
        debug!(count = calls.len(), "Executing tool calls");

        let mut results = Vec::new();

        for call in calls {
            let output = self
                .tool_registry
                .execute(call.name(), call.arguments().clone())
                .await;

            let result = match output {
                Ok(value) => {
                    debug!(tool = call.name(), "Tool executed successfully");
                    ToolResult {
                        tool_call_id: call.id().clone(),
                        output: value,
                        is_error: false,
                        error_message: None,
                    }
                }
                Err(e) => {
                    error!(tool = call.name(), error = %e, "Tool execution failed");
                    ToolResult {
                        tool_call_id: call.id().clone(),
                        output: serde_json::json!(null),
                        is_error: true,
                        error_message: Some(e.to_string()),
                    }
                }
            };

            results.push(result);
        }

        debug!(count = results.len(), "Tool execution complete");
        Ok(results)
    }
}
