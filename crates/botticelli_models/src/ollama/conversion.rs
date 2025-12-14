//! Type conversions between Ollama and Botticelli types.

use botticelli_core::{Input, Message, Output, Role};
use ollama_rs::generation::completion::GenerationResponse;

/// Convert Botticelli messages to Ollama prompt.
#[tracing::instrument(skip(messages), fields(message_count = messages.len()))]
pub fn messages_to_prompt(messages: &[Message]) -> String {
    let mut prompt = String::new();

    for msg in messages {
        let role_prefix = match msg.role() {
            Role::User => "User: ",
            Role::Assistant => "Assistant: ",
            Role::System => "System: ",
        };

        prompt.push_str(role_prefix);

        for input in msg.content() {
            match input {
                Input::Text(text) => {
                    prompt.push_str(text);
                    prompt.push('\n');
                }
                Input::Image { .. }
                | Input::Audio { .. }
                | Input::Video { .. }
                | Input::Document { .. }
                | Input::BotCommand { .. }
                | Input::Table { .. }
                | Input::Narrative { .. } => {
                    // Ollama supports vision models, but handle separately
                    prompt.push_str("[Media/Data content]\n");
                }
                Input::ToolCall { .. } | Input::ToolResult { .. } => {
                    // Ollama doesn't natively support tool calling in this format
                    prompt.push_str("[Tool interaction]\n");
                }
            }
        }

        prompt.push('\n');
    }

    prompt
}

/// Convert Ollama response to Botticelli Output.
#[tracing::instrument(skip(response))]
pub fn response_to_output(response: GenerationResponse) -> Output {
    Output::Text(response.response)
}
