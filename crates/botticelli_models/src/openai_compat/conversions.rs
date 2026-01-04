//! Type conversions between Botticelli and OpenAI formats.

use botticelli_error::{OpenAICompatError, OpenAICompatErrorKind};
use crate::openai_compat::{ChatMessage, ChatRequest, ChatResponse};
use botticelli_core::{GenerateRequest, GenerateResponse, Input, Output};

/// Converts a Botticelli GenerateRequest to OpenAI chat format.
pub fn to_chat_request(
    req: &GenerateRequest,
    model: &str,
) -> Result<ChatRequest, OpenAICompatError> {
    let mut messages = Vec::new();

    for msg in req.messages() {
        let role = match msg.role() {
            botticelli_core::Role::User => "user",
            botticelli_core::Role::Assistant => "assistant",
            botticelli_core::Role::System => "system",
        };

        for content in msg.content() {
            match content {
                Input::Text(text) => {
                    messages.push(ChatMessage::new(
                        role.to_string(),
                        text.clone(),
                        None,
                    ));
                }
                _ => {
                    return Err(OpenAICompatErrorKind::InvalidRequest(
                        "Only text inputs supported in OpenAI format".to_string(),
                    ).into());
                }
            }
        }
    }

    let mut builder = ChatRequest::builder();
    builder.model(model.to_string()).messages(messages);

    if let Some(max_tokens) = req.max_tokens() {
        builder.max_tokens(*max_tokens);
    }

    if let Some(temp) = req.temperature() {
        builder.temperature(*temp);
    }

    builder
        .build()
        .map_err(|e| OpenAICompatErrorKind::Builder(format!("Failed to build request: {}", e)).into())
}

/// Converts an OpenAI chat response to Botticelli GenerateResponse.
pub fn from_chat_response(response: &ChatResponse) -> Result<GenerateResponse, OpenAICompatError> {
    let content = response
        .choices()
        .first()
        .map(|choice| choice.message().content().clone())
        .ok_or_else(|| OpenAICompatErrorKind::InvalidRequest(
            "No choices in response".to_string()
        ))?;

    let output = Output::Text(content);

    // Extract token usage if available
    let usage =
        response.usage().as_ref().and_then(|u| {
            match (u.prompt_tokens(), u.completion_tokens(), u.total_tokens()) {
                (Some(input), Some(output), Some(total)) => Some(
                    botticelli_core::TokenUsageData::new(*input as u64, *output as u64, *total as u64),
                ),
                _ => None,
            }
        });

    // Map finish_reason to StopReason
    let stop_reason = response
        .choices()
        .first()
        .and_then(|choice| choice.finish_reason().as_ref())
        .map(|reason| match reason.as_str() {
            "stop" => botticelli_core::StopReason::EndTurn,
            "length" => botticelli_core::StopReason::MaxTokens,
            "tool_calls" | "function_call" => botticelli_core::StopReason::ToolUse,
            "content_filter" => botticelli_core::StopReason::ContentFilter,
            _ => botticelli_core::StopReason::Other,
        })
        .unwrap_or(botticelli_core::StopReason::EndTurn);

    GenerateResponse::builder()
        .outputs(vec![output])
        .stop_reason(stop_reason)
        .usage(usage)
        .build()
        .map_err(|e| OpenAICompatErrorKind::Builder(format!("Failed to build response: {}", e)).into())
}
