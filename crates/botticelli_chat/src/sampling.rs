use botticelli_core::{
    GenerateRequest, GenerateResponse, Input, Message, Role, ToolDefinition, ToolResult,
};
use botticelli_interface::{BotticelliDriver, ToolCalling};
use botticelli_mcp::{
    ConversationSession, ConversationTurn, LlmSampler, SamplingError, SamplingErrorKind,
    ToolRegistry,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

/// LLM sampler implementation for chat system.
pub struct ChatLlmSampler {
    /// LLM provider with tool calling support
    provider: Arc<RwLock<Arc<dyn ToolCalling>>>,

    /// Tool registry for execution
    tool_registry: Arc<ToolRegistry>,

    /// Chat session for model selection and fallback
    chat_session: Arc<RwLock<crate::ChatSession>>,

    /// Service container for creating new clients
    services: Arc<crate::ServiceContainer>,
}

impl ChatLlmSampler {
    /// Create new sampler with provider, tool registry, and fallback support.
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider that implements ToolCalling trait
    /// * `tool_registry` - Registry of available MCP tools
    /// * `chat_session` - Session tracking current model with fallback logic
    /// * `services` - Service container for creating new clients on fallback
    pub fn new(
        provider: Arc<dyn ToolCalling>,
        tool_registry: Arc<ToolRegistry>,
        chat_session: Arc<tokio::sync::RwLock<crate::ChatSession>>,
        services: Arc<crate::ServiceContainer>,
    ) -> Self {
        Self {
            provider: Arc::new(tokio::sync::RwLock::new(provider)),
            tool_registry,
            chat_session,
            services,
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
                            tool_call_id: result.tool_call_id().clone(),
                            content: result.content().to_string(),
                            is_error: *result.is_error(),
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
    #[instrument(skip(self, session, available_tools), fields(tool_count = available_tools.len()))]
    async fn generate(
        &self,
        session: &ConversationSession,
        available_tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, SamplingError> {
        debug!(
            turn_count = session.turn_count(),
            tool_count = available_tools.len(),
            "Generating next response"
        );

        // Build request from session
        let request = self.build_request(session)?;

        // Retry loop with fallback on rate limits
        const MAX_RETRIES: usize = 3;
        let mut attempts = 0;

        loop {
            attempts += 1;

            // Get current provider
            let provider = {
                let guard = self.provider.read().await;
                guard.clone()
            };

            // Try to generate
            let response = if available_tools.is_empty() {
                debug!("No tools available, using base generate");
                provider.generate(&request).await
            } else {
                debug!(tool_count = available_tools.len(), "Generating with tools");
                provider.generate_with_tools(&request, available_tools).await
            };

            match response {
                Ok(resp) => {
                    debug!(output_count = resp.outputs().len(), "Received response");
                    return Ok(resp);
                }
                Err(e) => {
                    let error_msg = e.to_string();

                    // Try fallback if this looks like a rate limit error and we have retries left
                    if attempts < MAX_RETRIES {
                        let mut session_guard = self.chat_session.write().await;

                        match session_guard.handle_rate_limit(&error_msg) {
                            Ok(next_model) => {
                                info!(
                                    current_model = ?session_guard.current_model(),
                                    next_model = ?next_model,
                                    attempt = attempts,
                                    "Rate limit hit, falling back to next model"
                                );

                                // Create new client for fallback model
                                match self.services.create_tool_calling_client(next_model) {
                                    Ok(new_provider) => {
                                        // Update provider with new client
                                        let mut provider_guard = self.provider.write().await;
                                        *provider_guard = new_provider;
                                        drop(provider_guard);
                                        drop(session_guard);

                                        debug!("Retrying with fallback provider");
                                        continue; // Retry with new provider
                                    }
                                    Err(create_err) => {
                                        error!(error = %create_err, "Failed to create fallback client");
                                        return Err(SamplingError::new(
                                            SamplingErrorKind::ProviderError(format!(
                                                "Fallback failed: {}",
                                                create_err
                                            )),
                                        ));
                                    }
                                }
                            }
                            Err(_) => {
                                // Not a rate limit error or no fallback available
                                warn!(
                                    attempts,
                                    "No fallback available or not a rate limit error"
                                );
                                return Err(SamplingError::new(SamplingErrorKind::ProviderError(
                                    error_msg,
                                )));
                            }
                        }
                    } else {
                        error!(attempts, "Max retries exceeded");
                        return Err(SamplingError::new(SamplingErrorKind::ProviderError(
                            format!("Max retries ({}) exceeded: {}", MAX_RETRIES, error_msg),
                        )));
                    }
                }
            }
        }
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
                    ToolResult::new(call.id().clone(), value, false)
                }
                Err(e) => {
                    error!(tool = call.name(), error = %e, "Tool execution failed");
                    ToolResult::new(call.id().clone(), serde_json::json!(null), true)
                }
            };

            results.push(result);
        }

        debug!(count = results.len(), "Tool execution complete");
        Ok(results)
    }
}
