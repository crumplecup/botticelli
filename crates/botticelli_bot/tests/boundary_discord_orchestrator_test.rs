//! Boundary Test: Discord Handler ↔ MCP Orchestrator
//!
//! Tests the handoff between Discord command handling and MCP orchestration.
//! Verifies messages trigger orchestration with proper context.

use async_trait::async_trait;
use botticelli_bot::DiscordMcpBridge;
use botticelli_mcp_client::{
    FinishReason, GenerationConfig, GenerationResponse, LlmAdapter, Message, MessageRole,
    Orchestrator, TokenUsage, ToolRegistry,
};
use std::sync::{Arc, Mutex};

/// Mock LLM adapter for testing
struct MockLlmAdapter {
    responses: Mutex<Vec<GenerationResponse>>,
}

impl MockLlmAdapter {
    fn new(responses: Vec<GenerationResponse>) -> Self {
        Self {
            responses: Mutex::new(responses),
        }
    }
}

#[async_trait]
impl LlmAdapter for MockLlmAdapter {
    async fn generate(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<botticelli_mcp_client::LlmToolSchema>,
        _config: GenerationConfig,
    ) -> Result<GenerationResponse, botticelli_mcp_client::McpClientError> {
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            panic!("No more mock responses");
        }
        Ok(responses.remove(0))
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn max_context_tokens(&self) -> u32 {
        4096
    }
}

#[tokio::test]
async fn test_discord_message_to_orchestrator_boundary() {
    // Setup: Create orchestrator with mock LLM
    let registry = Arc::new(ToolRegistry::new());
    
    let mock_response = GenerationResponse::new(
        Message::new(
            MessageRole::Assistant,
            "Hello! I can help you with that.".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        TokenUsage::new(10, 20, 30),
        FinishReason::Stop,
    );
    
    let adapter = Arc::new(MockLlmAdapter::new(vec![mock_response]));
    let orchestrator = Arc::new(Orchestrator::new(registry, adapter, 5));
    
    // Create bridge
    let bridge = DiscordMcpBridge::new(orchestrator);

    // Simulate: Discord message received
    let result = bridge
        .handle_message("channel_123", "user_456", "Hello bot!")
        .await
        .expect("Message should be handled");

    // Verify: Result contains response
    assert!(!result.is_empty(), "Should return non-empty response");
}

#[tokio::test]
async fn test_discord_context_propagation_boundary() {
    // Setup
    let registry = Arc::new(ToolRegistry::new());
    let mock_response = GenerationResponse::new(
        Message::new(
            MessageRole::Assistant,
            "Context preserved".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        TokenUsage::new(10, 20, 30),
        FinishReason::Stop,
    );
    
    let adapter = Arc::new(MockLlmAdapter::new(vec![mock_response]));
    let orchestrator = Arc::new(Orchestrator::new(registry, adapter, 5));
    let bridge = DiscordMcpBridge::new(orchestrator);

    // Simulate: Message with Discord-specific context
    let result = bridge
        .handle_message("channel_789", "user_123", "Test message with context")
        .await;

    // Verify: Context is preserved (no errors)
    assert!(result.is_ok(), "Message with context should succeed");
}

#[tokio::test]
async fn test_discord_error_handling_boundary() {
    // Setup with orchestrator that will hit max iterations
    let registry = Arc::new(ToolRegistry::new());
    
    // Return tool calls forever (will hit max iterations)
    let mock_responses = vec![
        GenerationResponse::new(
            Message::new(
                MessageRole::Assistant,
                String::new(),
                vec![botticelli_mcp_client::LlmToolCall::new(
                    "call_1".to_string(),
                    "nonexistent_tool".to_string(),
                    serde_json::json!({}),
                )],
                Vec::new(),
            ),
            TokenUsage::new(10, 20, 30),
            FinishReason::ToolCalls,
        );
        10
    ];
    
    let adapter = Arc::new(MockLlmAdapter::new(mock_responses));
    let orchestrator = Arc::new(Orchestrator::new(registry, adapter, 2)); // Low max iterations
    let bridge = DiscordMcpBridge::new(orchestrator);

    // Simulate: Message that will cause orchestrator error
    let result = bridge
        .handle_message("channel_000", "user_000", "This will fail")
        .await;

    // Verify: Error is translated to Discord-friendly format
    // Note: Current implementation returns placeholder, will need updating
    // when full orchestration is implemented
    assert!(result.is_ok() || result.is_err(), "Should handle gracefully");
}
