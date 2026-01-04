//! Integration tests for Anthropic tool calling.
//!
//! These tests use the real Anthropic API and are rate-limited.
//! Run with: `just test-api`

#![cfg(feature = "anthropic")]

use botticelli_core::{GenerateRequest, Input, Message, Role, ToolDefinition};
use botticelli_error::BotticelliResult;
use botticelli_interface::ToolCalling;
use botticelli_models::AnthropicClient;
use serde_json::json;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_anthropic_tool_calling() -> BotticelliResult<()> {
    // Get API key from environment
    let api_key =
        std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set for API tests");

    // Create client with latest Sonnet model
    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");

    // Define a simple echo tool (using interface ToolDefinition)
    let tool = ToolDefinition::new(
        "echo".to_string(),
        "Echoes back the input message".to_string(),
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo back"
                }
            },
            "required": ["message"]
        }),
    );

    // Create a minimal request asking to use the tool
    let message = Message::new(
        Role::User,
        vec![Input::Text("Use echo to say 'Hi'".to_string())],
    );

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(100u32)
        .build()
        .expect("Valid request"); // Minimal tokens to conserve rate limits

    // Use ToolCalling trait - tools passed as explicit parameter
    let response = client
        .generate_with_tools(&request, &[tool])
        .await?;

    // Verify we got tool calls in the response
    let has_tool_call = response
        .outputs()
        .iter()
        .any(|o| matches!(o, botticelli_core::Output::ToolCalls(_)));

    assert!(
        has_tool_call,
        "Response should contain at least one tool call"
    );

    // Verify the stop reason is ToolUse
    assert_eq!(
        response.stop_reason(),
        &botticelli_core::StopReason::ToolUse,
        "Stop reason should be ToolUse when model calls a tool"
    );

    // Extract and verify the tool call details
    for output in response.outputs() {
        if let botticelli_core::Output::ToolCalls(calls) = output {
            for call in calls {
                assert_eq!(call.name(), "echo", "Tool name should be 'echo'");
                assert!(
                    call.arguments().get("message").is_some(),
                    "Tool call should have 'message' argument"
                );
            }
        }
    }

    Ok(())
}
