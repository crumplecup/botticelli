//! Tests for ChatLlmSampler with mock providers.

#![cfg(feature = "cli")]

use botticelli_chat::ChatLlmSampler;
use botticelli_core::{
    GenerateRequest, GenerateResponse, GenerateResponseBuilder, Output, ToolCall, ToolDefinition,
};
use botticelli_error::ProviderError;
use botticelli_interface::LlmProvider;
use botticelli_mcp::{ConversationSession, ConversationTurn, LlmSampler, ToolRegistry};
use std::sync::Arc;

/// Mock provider that returns a simple text response.
struct MockTextProvider {
    response_text: String,
}

impl MockTextProvider {
    fn new(text: impl Into<String>) -> Self {
        Self {
            response_text: text.into(),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockTextProvider {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = ProviderError;

    async fn generate(&self, _request: &Self::Request) -> Result<Self::Response, Self::Error> {
        Ok(GenerateResponseBuilder::default()
            .outputs(vec![Output::Text(self.response_text.clone())])
            .stop_reason(botticelli_core::StopReason::EndTurn)
            .build()
            .expect("Valid response"))
    }

    fn provider_name(&self) -> &str {
        "mock-text"
    }

    fn default_model(&self) -> &str {
        "test-model"
    }

    fn supports_tools(&self) -> bool {
        false
    }
}

/// Mock provider that returns tool calls.
struct MockToolProvider {
    tool_name: String,
    tool_args: serde_json::Value,
}

impl MockToolProvider {
    fn new(tool_name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            tool_name: tool_name.into(),
            tool_args: args,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockToolProvider {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = ProviderError;

    async fn generate(&self, _request: &Self::Request) -> Result<Self::Response, Self::Error> {
        let tool_call = ToolCall::new(
            "call_123".to_string(),
            self.tool_name.clone(),
            self.tool_args.clone(),
        );

        Ok(GenerateResponseBuilder::default()
            .outputs(vec![Output::ToolCalls(vec![tool_call])])
            .stop_reason(botticelli_core::StopReason::ToolUse)
            .build()
            .expect("Valid response"))
    }

    fn provider_name(&self) -> &str {
        "mock-tool"
    }

    fn default_model(&self) -> &str {
        "test-model"
    }

    fn supports_tools(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_sampler_with_text_response() {
    let provider = Arc::new(MockTextProvider::new("Hello, world!"));
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = ChatLlmSampler::new(provider, tool_registry);

    let mut session = ConversationSession::new("You are helpful");
    session.add_turn(ConversationTurn::UserMessage {
        content: "Hi".to_string(),
        attachments: None,
    });

    let tools: Vec<ToolDefinition> = vec![];
    let result = sampler.generate(&session, &tools).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.outputs().len(), 1);

    match &response.outputs()[0] {
        Output::Text(text) => assert_eq!(text, "Hello, world!"),
        _ => panic!("Expected text output"),
    }
}

#[tokio::test]
async fn test_sampler_builds_request_from_session() {
    let provider = Arc::new(MockTextProvider::new("Response"));
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = ChatLlmSampler::new(provider, tool_registry);

    let mut session = ConversationSession::new("System prompt");

    // Add multiple turns
    session.add_turn(ConversationTurn::UserMessage {
        content: "First message".to_string(),
        attachments: None,
    });

    session.add_turn(ConversationTurn::AssistantMessage {
        content: "First response".to_string(),
    });

    session.add_turn(ConversationTurn::UserMessage {
        content: "Second message".to_string(),
        attachments: None,
    });

    let tools: Vec<ToolDefinition> = vec![];
    let result = sampler.generate(&session, &tools).await;

    assert!(result.is_ok());
    // If we got here, request building succeeded
}

#[tokio::test]
async fn test_sampler_with_tool_call() {
    let provider = Arc::new(MockToolProvider::new(
        "echo",
        serde_json::json!({"message": "test"}),
    ));
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = ChatLlmSampler::new(provider, tool_registry);

    let mut session = ConversationSession::new("System");
    session.add_turn(ConversationTurn::UserMessage {
        content: "Use the echo tool".to_string(),
        attachments: None,
    });

    let tools: Vec<ToolDefinition> = vec![];
    let result = sampler.generate(&session, &tools).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.outputs().len(), 1);

    match &response.outputs()[0] {
        Output::ToolCalls(calls) => {
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].name(), "echo");
        }
        _ => panic!("Expected tool call output"),
    }
}

#[tokio::test]
async fn test_execute_tools_with_registered_tool() {
    let provider = Arc::new(MockTextProvider::new("Done"));
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = ChatLlmSampler::new(provider, tool_registry);

    let tool_call = ToolCall::new(
        "call_456".to_string(),
        "echo".to_string(),
        serde_json::json!({"message": "hello"}),
    );

    let result = sampler.execute_tools(&[tool_call]).await;

    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].tool_call_id, "call_456");
}

#[tokio::test]
async fn test_execute_tools_with_nonexistent_tool() {
    let provider = Arc::new(MockTextProvider::new("Done"));
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = ChatLlmSampler::new(provider, tool_registry);

    let tool_call = ToolCall::new(
        "call_789".to_string(),
        "nonexistent_tool".to_string(),
        serde_json::json!({}),
    );

    let result = sampler.execute_tools(&[tool_call]).await;

    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].is_error);
    assert!(results[0].error_message.is_some());
}

#[tokio::test]
async fn test_full_sampling_loop_with_text() {
    let provider = Arc::new(MockTextProvider::new("Final answer"));
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry));

    let mut session = ConversationSession::new("You are helpful");
    session.add_turn(ConversationTurn::UserMessage {
        content: "Hello".to_string(),
        attachments: None,
    });

    let tools: Vec<ToolDefinition> = vec![];
    let result = sampler.sample(&mut session, &tools).await;

    assert!(result.is_ok());
    let sampling_result = result.unwrap();

    match sampling_result {
        botticelli_mcp::SamplingResult::Completed { final_response } => {
            assert_eq!(final_response, "Final answer");
        }
    }

    // Session should have assistant response added
    assert_eq!(session.turn_count(), 2);
}
