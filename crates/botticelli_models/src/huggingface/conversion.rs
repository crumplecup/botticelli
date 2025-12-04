//! Type conversions between Botticelli and HuggingFace types.

use botticelli_core::{GenerateRequest, GenerateResponse, Input, Message, Output, Role};
use botticelli_error::{HuggingFaceErrorKind, ModelsError, ModelsResult};

use super::dto::{HuggingFaceRequest, HuggingFaceResponse, HuggingFaceRole};

/// Converts Botticelli GenerateRequest to HuggingFace request.
pub fn to_huggingface_request(
    request: &GenerateRequest,
    model: &str,
) -> ModelsResult<HuggingFaceRequest> {
    // Convert messages to a single input string
    let inputs = messages_to_string(request.messages())?;

    let mut builder = HuggingFaceRequest::builder();
    builder.model(model.to_string()).inputs(inputs);

    if let Some(max_tokens) = request.max_tokens() {
        builder.max_new_tokens(*max_tokens as usize);
    }

    if let Some(temp) = request.temperature() {
        builder.temperature(*temp);
    }

    builder.build().map_err(|e| {
        ModelsError::new(botticelli_error::ModelsErrorKind::Builder(format!(
            "Failed to build HuggingFace request: {}",
            e
        )))
    })
}

/// Converts HuggingFace response to Botticelli GenerateResponse.
pub fn from_huggingface_response(response: &HuggingFaceResponse) -> ModelsResult<GenerateResponse> {
    let output = Output::Text(response.generated_text().clone());

    GenerateResponse::builder()
        .outputs(vec![output])
        .build()
        .map_err(|e| {
            ModelsError::new(botticelli_error::ModelsErrorKind::Builder(format!(
                "Failed to build GenerateResponse: {}",
                e
            )))
        })
}

/// Converts messages to a single input string for HuggingFace.
fn messages_to_string(messages: &[Message]) -> ModelsResult<String> {
    let mut result = String::new();

    for message in messages {
        let role_prefix = match message.role() {
            Role::User => "User: ",
            Role::Assistant => "Assistant: ",
            Role::System => "System: ",
        };

        result.push_str(role_prefix);

        for input in message.content() {
            match input {
                Input::Text(text) => {
                    result.push_str(text);
                    result.push('\n');
                }
                Input::Image { .. }
                | Input::Audio { .. }
                | Input::Video { .. }
                | Input::Document { .. }
                | Input::BotCommand { .. }
                | Input::Table { .. }
                | Input::Narrative { .. } => {
                    return Err(ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                        HuggingFaceErrorKind::Unsupported(
                            "Only text inputs supported by HuggingFace text generation".to_string(),
                        ),
                    )));
                }
            }
        }
    }

    Ok(result)
}

/// Converts Botticelli Role to HuggingFace role.
#[allow(dead_code)]
fn to_huggingface_role(role: &Role) -> ModelsResult<HuggingFaceRole> {
    match role {
        Role::User => Ok(HuggingFaceRole::User),
        Role::Assistant => Ok(HuggingFaceRole::Assistant),
        Role::System => Ok(HuggingFaceRole::System),
    }
}
