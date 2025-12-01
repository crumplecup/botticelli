//! Conversion between botticelli and server API types

use botticelli_core::{GenerateRequest, GenerateResponse, Input, Message, Output};
#[cfg(feature = "models")]
use botticelli_error::{ModelsError, ModelsErrorKind};
use botticelli_error::{ServerError, ServerErrorKind};
use botticelli_interface::{FinishReason, StreamChunk};

use crate::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse,
    request::{ChatCompletionRequestBuilder, MessageBuilder},
};

/// Convert botticelli GenerateRequest to server ChatCompletionRequest
#[tracing::instrument(skip(request))]
pub fn to_chat_request(
    request: GenerateRequest,
    model: String,
) -> Result<ChatCompletionRequest, ServerError> {
    let messages = request
        .messages()
        .iter()
        .map(|m| message_to_server_message(m.clone()))
        .collect::<Result<Vec<_>, _>>()?;

    let chat_request = ChatCompletionRequestBuilder::default()
        .model(model)
        .messages(messages)
        .max_tokens(*request.max_tokens())
        .temperature(*request.temperature())
        .top_p(None)
        .stream(Some(false))
        .build()
        .map_err(|e| {
            ServerError::new(ServerErrorKind::Api(format!(
                "Failed to build request: {}",
                e
            )))
        })?;

    Ok(chat_request)
}

/// Convert Message to server Message
fn message_to_server_message(msg: Message) -> Result<crate::Message, ServerError> {
    // Extract text from content (handle multimodal inputs)
    let text = msg
        .content()
        .iter()
        .filter_map(|input| match input {
            Input::Text(t) => Some(t.as_str()),
            // Skip non-text inputs for text-only server
            Input::Image { .. }
            | Input::Audio { .. }
            | Input::Video { .. }
            | Input::Document { .. } => None,
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

    let role = match msg.role() {
        botticelli_core::Role::User => "user",
        botticelli_core::Role::Assistant => "assistant",
        botticelli_core::Role::System => "system",
    };

    MessageBuilder::default()
        .role(role)
        .content(text)
        .build()
        .map_err(|e| {
            ServerError::new(ServerErrorKind::Api(format!(
                "Failed to build message: {}",
                e
            )))
        })
}

/// Convert ChatCompletionResponse to GenerateResponse
#[tracing::instrument(skip(response))]
pub fn from_chat_response(
    response: ChatCompletionResponse,
) -> Result<GenerateResponse, ServerError> {
    let choice = response
        .choices()
        .first()
        .ok_or_else(|| ServerError::new(ServerErrorKind::Api("No choices in response".into())))?;

    let text = choice.message().content().clone();
    let output = Output::Text(text);

    GenerateResponse::builder()
        .outputs(vec![output])
        .build()
        .map_err(|e| {
            #[cfg(feature = "models")]
            {
                let models_error = ModelsError::new(ModelsErrorKind::Builder(e.to_string()));
                ServerError::new(ServerErrorKind::Models(models_error))
            }
            #[cfg(not(feature = "models"))]
            {
                ServerError::new(ServerErrorKind::Api(format!(
                    "Failed to build response: {}",
                    e
                )))
            }
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
    let choice = chunk
        .choices()
        .first()
        .ok_or_else(|| ServerError::new(ServerErrorKind::Api("No choices in chunk".into())))?;

    let text = choice.delta().content().clone().unwrap_or_default();
    let is_final = choice.finish_reason().is_some();
    let finish_reason = choice
        .finish_reason()
        .as_ref()
        .map(|r| map_finish_reason(r));

    StreamChunk::builder()
        .content(Output::Text(text))
        .is_final(is_final)
        .finish_reason(finish_reason)
        .build()
        .map_err(|e| {
            #[cfg(feature = "models")]
            {
                let models_error = ModelsError::new(ModelsErrorKind::Builder(e.to_string()));
                ServerError::new(ServerErrorKind::Models(models_error))
            }
            #[cfg(not(feature = "models"))]
            {
                ServerError::new(ServerErrorKind::Api(format!(
                    "Failed to build chunk: {}",
                    e
                )))
            }
        })
}
