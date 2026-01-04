//! ToolCalling implementation for OpenAI-compatible clients.

use crate::openai_compat::{ChatFunctionDef, ChatTool, OpenAICompatibleClient, conversions};
use async_trait::async_trait;
use botticelli_core::{Capabilities, GenerateRequest, GenerateResponse, Output, ToolCall, ToolDefinition};
use botticelli_error::{BotticelliError, BotticelliResult, OpenAICompatErrorKind};
use botticelli_interface::{BotticelliDriver, ToolCalling};

#[async_trait]
impl BotticelliDriver for OpenAICompatibleClient {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = botticelli_error::BotticelliError;
    type RateLimitConfig = botticelli_rate_limit::RateLimitConfig;
    type Capabilities = Capabilities;

    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        self.generate(req).await.map_err(BotticelliError::from)
    }

    fn provider_name(&self) -> &str {
        self.provider_name()
    }

    fn model_name(&self) -> &str {
        self.model()
    }

    fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig {
        self.rate_limits()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
            .with_streaming(false)
            .with_tool_calling(true)
            .with_vision(false)
            .with_audio(false)
            .with_video(false)
            .with_embeddings(false)
            .with_json_mode(false)
            .with_batch_generation(false)
    }
}

#[async_trait]
impl ToolCalling for OpenAICompatibleClient {
    type Error = BotticelliError;
    type ToolDefinition = ToolDefinition;

    #[tracing::instrument(skip(self, request), fields(provider = self.provider_name(), model = self.model_name()))]
    async fn generate_with_tools(
        &self,
        request: &GenerateRequest,
        tools: &[ToolDefinition],
    ) -> BotticelliResult<GenerateResponse> {
        tracing::info!(
            tool_count = tools.len(),
            provider = self.provider_name(),
            model = self.model_name(),
            "Starting generate_with_tools"
        );

        tracing::debug!(
            tool_names = ?tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
            "Tool definitions"
        );

        // Convert tools to OpenAI format
        let chat_tools: Vec<ChatTool> = tools
            .iter()
            .map(|tool| ChatTool::new(
                "function".to_string(),
                ChatFunctionDef::new(
                    tool.name().to_string(),
                    tool.description().to_string(),
                    tool.input_schema().clone(),
                )
            ))
            .collect();

        // Build request with tools
        let chat_request_base = conversions::to_chat_request(request, self.model_name())?;

        // Add tools to request
        let chat_request = crate::openai_compat::ChatRequest::builder()
            .model(chat_request_base.model().to_string())
            .messages(chat_request_base.messages().clone())
            .max_tokens(*chat_request_base.max_tokens())
            .temperature(*chat_request_base.temperature())
            .stream(*chat_request_base.stream())
            .tools(Some(chat_tools))
            .build()
            .map_err(|e| botticelli_error::OpenAICompatError::new(
                botticelli_error::OpenAICompatErrorKind::Builder(e.to_string())
            ))?;

        tracing::info!("Sending HTTP request to provider with tools");

        // Send request (use existing client logic)
        let response = self.generate_internal(&chat_request).await.map_err(|e| {
            tracing::error!(error = %e, "HTTP request failed");
            BotticelliError::from(e)
        })?;

        tracing::info!("Received response from provider");

        // Check if response contains tool calls
        if let Some(tool_calls) = response
            .choices()
            .first()
            .and_then(|choice| choice.message().tool_calls().as_ref())
        {
            let parsed_calls: Vec<ToolCall> = tool_calls
                .iter()
                .map(|call| {
                    // Parse JSON string arguments
                    let args: serde_json::Value = serde_json::from_str(call.function().arguments())
                        .unwrap_or_else(|_| serde_json::json!({}));

                    ToolCall::new(call.id().clone(), call.function().name().clone(), args)
                })
                .collect();

            tracing::info!(
                tool_call_count = parsed_calls.len(),
                "Response contains tool calls"
            );

            GenerateResponse::builder()
                .outputs(vec![Output::ToolCalls(parsed_calls)])
                .stop_reason(botticelli_core::StopReason::ToolUse)
                .build()
                .map_err(|e| OpenAICompatErrorKind::Builder(e.to_string()).into())
        } else {
            tracing::info!("Response contains text, no tool calls");
            // No tool calls, convert to text response
            conversions::from_chat_response(&response).map_err(BotticelliError::from)
        }
    }
}
