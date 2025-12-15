use crate::llm_adapter::{
    FinishReason, GenerationConfig, GenerationResponse, LlmAdapter, Message, MessageRole,
    TokenUsage, ToolSchema,
};
use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, Input, Message as CoreMessage, Output, Role, StopReason};
use botticelli_interface::BotticelliDriver;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Bridges the `LlmAdapter` trait to any `BotticelliDriver` implementation.
///
/// This allows using existing Botticelli LLM implementations (AnthropicClient,
/// GeminiClient, etc.) with the MCP orchestration layer.
pub struct DriverAdapter {
    driver: Arc<dyn BotticelliDriver>,
}

impl DriverAdapter {
    /// Create a new adapter wrapping a Botticelli driver.
    pub fn new(driver: Arc<dyn BotticelliDriver>) -> Self {
        Self { driver }
    }
}

#[async_trait]
impl LlmAdapter for DriverAdapter {
    #[instrument(skip(self, messages, tools, _config))]
    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        _config: GenerationConfig,
    ) -> McpClientResult<GenerationResponse> {
        debug!(
            message_count = messages.len(),
            tool_count = tools.len(),
            "Converting messages to core format"
        );

        // Convert messages to core format
        let core_messages: Vec<CoreMessage> = messages
            .iter()
            .filter_map(convert_message_to_core)
            .collect();

        // Create request
        let request = GenerateRequest::builder()
            .messages(core_messages)
            .build()
            .map_err(|e| {
                McpClientError::new(McpClientErrorKind::LlmError(format!(
                    "Failed to build request: {}",
                    e
                )))
            })?;

        // Call driver
        debug!("Calling underlying driver");
        let response = self
            .driver
            .generate(&request)
            .await
            .map_err(|e| McpClientError::new(McpClientErrorKind::LlmError(e.to_string())))?;

        // Convert response
        debug!("Converting response from core format");
        convert_response_from_core(response)
    }

    fn model_name(&self) -> &str {
        self.driver.model_name()
    }

    fn supports_tools(&self) -> bool {
        // TODO: Check if driver implements tool calling
        // For now, assume true if provider supports it
        matches!(
            self.driver.provider_name(),
            "anthropic" | "openai" | "gemini"
        )
    }

    fn max_context_tokens(&self) -> u32 {
        // TODO: Get from driver metadata
        match self.driver.provider_name() {
            "anthropic" => 200_000,
            "gemini" => 1_000_000,
            "openai" => 128_000,
            "groq" => 8_192,
            _ => 4_096,
        }
    }
}

/// Convert MCP message to core message format.
fn convert_message_to_core(msg: &Message) -> Option<CoreMessage> {
    let role = match msg.role() {
        MessageRole::User => Role::User,
        MessageRole::Assistant => Role::Assistant,
        MessageRole::System => return None, // System handled separately
        MessageRole::Tool => return None,   // Tool results embedded differently
    };

    let inputs = vec![Input::Text(msg.content().clone())];

    Some(
        CoreMessage::builder()
            .role(role)
            .content(inputs)
            .build()
            .expect("Valid message"),
    )
}

/// Convert core response to MCP format.
fn convert_response_from_core(
    response: botticelli_core::GenerateResponse,
) -> McpClientResult<GenerationResponse> {
    // Get first output (for now, handle single output)
    let outputs = response.outputs();
    let first_output = outputs.first().ok_or_else(|| {
        McpClientError::new(McpClientErrorKind::LlmError(
            "No outputs in response".to_string(),
        ))
    })?;

    // Extract content and tool calls
    let (content, tool_calls, finish_reason) = match first_output {
        Output::Text(text) => (
            text.clone(),
            Vec::new(),
            convert_stop_reason(response.stop_reason()),
        ),
        Output::ToolCalls(calls) => {
            let llm_calls = calls
                .iter()
                .map(|call| {
                    crate::llm_adapter::ToolCall::new(
                        call.id().clone(),
                        call.name().clone(),
                        call.arguments().clone(),
                    )
                })
                .collect();
            (String::new(), llm_calls, FinishReason::ToolCalls)
        }
        Output::Json(val) => (
            serde_json::to_string(val).unwrap_or_default(),
            Vec::new(),
            convert_stop_reason(response.stop_reason()),
        ),
        _ => {
            return Err(McpClientError::new(McpClientErrorKind::LlmError(
                "Unsupported output type in adapter".to_string(),
            )));
        }
    };

    let message = Message::new(MessageRole::Assistant, content, tool_calls, Vec::new());

    // Extract usage if available
    let usage = if let Some(usage_data) = response.usage() {
        TokenUsage::new(
            *usage_data.input_tokens() as u32,
            *usage_data.output_tokens() as u32,
            *usage_data.total_tokens() as u32,
        )
    } else {
        TokenUsage::default()
    };

    Ok(GenerationResponse::new(message, usage, finish_reason))
}

/// Convert core StopReason to LlmAdapter FinishReason.
fn convert_stop_reason(reason: &StopReason) -> FinishReason {
    match reason {
        StopReason::EndTurn => FinishReason::Stop,
        StopReason::MaxTokens => FinishReason::MaxTokens,
        StopReason::ToolUse => FinishReason::ToolCalls,
        StopReason::ContentFilter => FinishReason::ContentFilter,
        StopReason::StopSequence => FinishReason::Stop,
        StopReason::Other => FinishReason::Error,
    }
}
