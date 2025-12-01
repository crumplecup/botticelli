//! Type conversions between Ollama and Botticelli types.

use botticelli_core::{Input, Message, Output, Role};
use ollama_rs::generation::completion::GenerationResponse;

/// Convert Botticelli messages to Ollama prompt.
pub fn messages_to_prompt(messages: &[Message]) -> String {
    let mut prompt = String::new();

    for msg in messages {
        let role_prefix = match msg.role() {
            Role::User => "User: ",
            Role::Model => "Assistant: ",
            Role::System => "System: ",
        };

        prompt.push_str(role_prefix);

        for input in msg.content() {
            match input {
                Input::Text(text) => {
                    prompt.push_str(text);
                    prompt.push('\n');
                }
                Input::Image(_) => {
                    // Ollama supports vision models, but handle separately
                    prompt.push_str("[Image content]\n");
                }
            }
        }

        prompt.push('\n');
    }

    prompt
}

/// Convert Ollama response to Botticelli Output.
pub fn response_to_output(response: GenerationResponse) -> Output {
    Output::Text(response.response)
}
