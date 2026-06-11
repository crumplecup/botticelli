//! Multi-backend LLM generation tools.
//!
//! This module provides tools for text generation using multiple LLM backends.
//! Each backend is feature-gated and implemented explicitly.

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_error::{McpError, McpResult};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_core::{GenerateRequest, Input, Message, Role};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_interface::BotticelliDriver;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use serde_json::{Value, json};

/// Common generation logic shared across all backends.
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
pub(crate) async fn execute_generation<D: BotticelliDriver + ?Sized>(
    driver: &D,
    input: Value,
    default_model: &str,
) -> McpResult<Value> {
    let prompt = input
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_input("Missing 'prompt'".to_string()))?;

    let model = input
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or(default_model);

    let max_tokens = input
        .get("max_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(1024) as u32;

    let temperature = input
        .get("temperature")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0) as f32;

    let system_prompt = input.get("system_prompt").and_then(|v| v.as_str());

    // Build messages
    let mut messages = Vec::new();

    if let Some(sys_prompt) = system_prompt {
        messages.push(
            Message::builder()
                .role(Role::User)
                .content(vec![Input::Text(sys_prompt.to_string())])
                .build()
                .map_err(|e| {
                    McpError::execution_failed(format!("Failed to build system message: {}", e))
                })?,
        );
    }

    messages.push(
        Message::builder()
            .role(Role::User)
            .content(vec![Input::Text(prompt.to_string())])
            .build()
            .map_err(|e| McpError::execution_failed(format!("Failed to build message: {}", e)))?,
    );

    // Build request
    let request = GenerateRequest::builder()
        .model(Some(model.to_string()))
        .messages(messages)
        .max_tokens(Some(max_tokens))
        .temperature(Some(temperature))
        .build()
        .map_err(|e| McpError::execution_failed(format!("Failed to build request: {}", e)))?;

    // Execute
    let response = driver
        .generate(&request)
        .await
        .map_err(|e| McpError::execution_failed(format!("Generation failed: {}", e)))?;

    // Extract text
    let text = response
        .outputs()
        .iter()
        .filter_map(|output| {
            if let botticelli_core::Output::Text(t) = output {
                Some(t.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(json!({
        "status": "success",
        "model": model,
        "text": text,
    }))
}
