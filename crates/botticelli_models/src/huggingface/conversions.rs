//! Type conversions between Botticelli and HuggingFace formats.

use crate::huggingface::{HuggingFaceParameters, HuggingFaceRequest, HuggingFaceResponse};
use botticelli_core::{GenerateRequest, GenerateResponse, Input, Output};
use botticelli_error::{HuggingFaceErrorKind, ModelsError, ModelsResult};

/// Converts Botticelli GenerateRequest to HuggingFace format.
pub fn to_huggingface_request(
    req: &GenerateRequest,
    model: &str,
) -> ModelsResult<HuggingFaceRequest> {
    let inputs = messages_to_text(req)?;

    let mut builder = HuggingFaceRequest::builder();
    builder.model(model.to_string()).inputs(inputs);

    if req.max_tokens().is_some() || req.temperature().is_some() {
        let mut params_builder = HuggingFaceParameters::builder();

        if let Some(max_tokens) = req.max_tokens() {
            params_builder.max_new_tokens(*max_tokens);
        }
        if let Some(temp) = req.temperature() {
            params_builder.temperature(*temp);
        }

        let params = params_builder.build().map_err(|e| {
            ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                HuggingFaceErrorKind::RequestConversion(format!(
                    "Failed to build parameters: {}",
                    e
                )),
            ))
        })?;
        builder.parameters(Some(params));
    }

    builder.build().map_err(|e| {
        ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
            HuggingFaceErrorKind::RequestConversion(format!("Failed to build request: {}", e)),
        ))
    })
}

/// Converts messages to plain text format.
fn messages_to_text(req: &GenerateRequest) -> ModelsResult<String> {
    let mut text = String::new();

    for message in req.messages() {
        let role_prefix = match message.role() {
            botticelli_core::Role::User => "User: ",
            botticelli_core::Role::Assistant => "Assistant: ",
            botticelli_core::Role::System => "System: ",
        };

        text.push_str(role_prefix);

        for input in message.content() {
            match input {
                Input::Text(s) => {
                    text.push_str(s);
                    text.push('\n');
                }
                _ => {
                    return Err(ModelsError::new(
                        botticelli_error::ModelsErrorKind::HuggingFace(
                            HuggingFaceErrorKind::RequestConversion(
                                "Only text inputs supported".to_string(),
                            ),
                        ),
                    ));
                }
            }
        }
    }

    Ok(text)
}

/// Converts HuggingFace response to Botticelli format.
pub fn from_huggingface_response(resp: &HuggingFaceResponse) -> ModelsResult<GenerateResponse> {
    let output = Output::Text(resp.generated_text().clone());

    GenerateResponse::builder()
        .outputs(vec![output])
        .build()
        .map_err(|e| {
            ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                HuggingFaceErrorKind::ResponseConversion(format!(
                    "Failed to build response: {}",
                    e
                )),
            ))
        })
}
