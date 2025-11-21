//! Conversion between botticelli and server API types

use botticelli_core::{GenerateRequest, GenerateResponse, Input, Message, Output};
use botticelli_error::{ServerError, ServerErrorKind};
use botticelli_interface::{FinishReason, StreamChunk};

use crate::{ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse};

/// Convert botticelli GenerateRequest to server ChatCompletionRequest
#[tracing::instrument(skip(request))]
pub fn to_chat_request(
    request: GenerateRequest,
    model: String,
) -> Result<ChatCompletionRequest, ServerError> {
    let messages = request
        .messages
        .into_iter()
        .map(message_to_server_message)
        .collect::<Result<Vec<_>, _>>()?;

    let chat_request = ChatCompletionRequest {
        model,
        messages,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        top_p: None,
        stream: Some(false),
    };

    Ok(chat_request)
}

/// Convert Message to server Message
fn message_to_server_message(msg: Message) -> Result<crate::Message, ServerError> {
    // Extract text from content (handle multimodal inputs)
    let text = msg
        .content
        .iter()
        .filter_map(|input| match input {
            Input::Text(t) => Some(t.as_str()),
            // Skip non-text inputs for text-only server
            Input::Image { .. } | Input::Audio { .. } | Input::Video { .. } | Input::Document { .. } => None,
            // Skip bot commands, table references, and narrative references (not supported in text-only server)
            Input::BotCommand { .. } | Input::Table { .. } | Input::Narrative { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        return Err(ServerError::new(ServerErrorKind::Api(
            "Message must contain text content".into(),
        )));
    }

    Ok(crate::Message {
        role: match msg.role {
            botticelli_core::Role::User => "user".to_string(),
            botticelli_core::Role::Assistant => "assistant".to_string(),
            botticelli_core::Role::System => "system".to_string(),
        },
        content: text,
    })
}

/// Convert ChatCompletionResponse to GenerateResponse
#[tracing::instrument(skip(response))]
pub fn from_chat_response(
    response: ChatCompletionResponse,
) -> Result<GenerateResponse, ServerError> {
    let choice = response.choices.first().ok_or_else(|| {
        ServerError::new(ServerErrorKind::Api("No choices in response".into()))
    })?;

    let text = choice.message.content.clone();
    let output = Output::Text(text);

    Ok(GenerateResponse {
        outputs: vec![output],
    })
}

/// Map OpenAI finish reason to botticelli FinishReason
fn map_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "content_filter" => FinishReason::ContentFilter,
        _ => FinishReason::Other,
    }
}

/// Convert streaming chunk to StreamChunk
pub fn chunk_to_stream_chunk(chunk: ChatCompletionChunk) -> Result<StreamChunk, ServerError> {
    let choice = chunk.choices.first().ok_or_else(|| {
        ServerError::new(ServerErrorKind::Api("No choices in chunk".into()))
    })?;

    let text = choice.delta.content.clone().unwrap_or_default();
    let is_final = choice.finish_reason.is_some();
    let finish_reason = choice.finish_reason.as_ref().map(|r| map_finish_reason(r));

    Ok(StreamChunk {
        content: Output::Text(text),
        is_final,
        finish_reason,
    })
}
