//! ToolCalling implementation for OpenAI-compatible clients.

use crate::openai_compat::{conversions, ChatFunctionDef, ChatTool, OpenAICompatibleClient};
use async_trait::async_trait;
use botticelli_error::{BackendError, BotticelliError, BotticelliResult};
use botticelli_core::{GenerateRequest, GenerateResponse, Output, ToolCall, ToolDefinition};
use botticelli_interface::{BotticelliDriver, Capabilities, ToolCalling};

#[async_trait]
impl BotticelliDriver for OpenAICompatibleClient {
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        self.generate(req)
            .await
            .map_err(|e| BotticelliError::from(BackendError::new(e.to_string())))
    }

    fn provider_name(&self) -> &'static str {
        self.provider_name()
    }

    fn model_name(&self) -> &str {
        self.model_name()
    }

    fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig {
        self.rate_limits()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            streaming: false,
            tool_calling: true,
            vision: false,
            audio: false,
            video: false,
            embeddings: false,
            json_mode: false,
            batch_generation: false,
        }
    }
}

#[async_trait]
impl ToolCalling for OpenAICompatibleClient {
    #[tracing::instrument(skip(self, request), fields(provider = self.provider_name()))]
    async fn generate_with_tools(
        &self,
        request: &GenerateRequest,
        tools: &[ToolDefinition],
    ) -> BotticelliResult<GenerateResponse> {
        tracing::debug!(
            tool_count = tools.len(),
            "Generating with tools"
        );

        // Convert tools to OpenAI format
        let chat_tools: Vec<ChatTool> = tools
            .iter()
            .map(|tool| ChatTool {
                tool_type: "function".to_string(),
                function: ChatFunctionDef {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: tool.input_schema().clone(),
                },
            })
            .collect();

        // Build request with tools
        let chat_request_base = conversions::to_chat_request(request, self.model_name())
            .map_err(|e| BotticelliError::from(BackendError::new(e.to_string())))?;

        // Add tools to request
        let chat_request = crate::openai_compat::ChatRequest::builder()
            .model(chat_request_base.model().to_string())
            .messages(chat_request_base.messages().clone())
            .max_tokens(*chat_request_base.max_tokens())
            .temperature(*chat_request_base.temperature())
            .stream(*chat_request_base.stream())
            .tools(Some(chat_tools))
            .build()
            .map_err(|e| BotticelliError::from(BackendError::new(e.to_string())))?;

        tracing::debug!("Sending request with tools");

        // Send request (use existing client logic)
        let response = self
            .generate_internal(&chat_request)
            .await
            .map_err(|e| BotticelliError::from(BackendError::new(e.to_string())))?;

        // Check if response contains tool calls
        if let Some(tool_calls) = response
            .choices
            .first()
            .and_then(|choice| choice.message.tool_calls.as_ref())
        {
            let parsed_calls: Vec<ToolCall> = tool_calls
                .iter()
                .map(|call| {
                    // Parse JSON string arguments
                    let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    
                    ToolCall::new(call.id.clone(), call.function.name.clone(), args)
                })
                .collect();

            tracing::debug!(tool_call_count = parsed_calls.len(), "Extracted tool calls");

            Ok(GenerateResponse::builder()
                .outputs(vec![Output::ToolCalls(parsed_calls)])
                .stop_reason(botticelli_core::StopReason::ToolUse)
                .build()
                .expect("Valid response"))
        } else {
            // No tool calls, convert to text response
            conversions::from_chat_response(&response)
                .map_err(|e| BotticelliError::from(BackendError::new(e.to_string())))
        }
    }
}
