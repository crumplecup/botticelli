//! End-to-end integration tests for sampling with tool calling.

use botticelli_chat::ChatLlmSampler;
use botticelli_core::{
    GenerateRequest, GenerateResponse, GenerateResponseBuilder, Output, StopReason, ToolCall,
};
use botticelli_interface::{LlmProvider, ProviderError};
use botticelli_mcp::{
    ConversationSession, ConversationTurn, LlmSampler, SamplingCoordinator, ToolRegistry,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Mock provider that simulates a multi-turn conversation with tool use.
///
/// First call: Returns tool calls
/// Second call: Returns final text response
struct MultiTurnMockProvider {
    call_count: Arc<Mutex<usize>>,
}

impl MultiTurnMockProvider {
    fn new() -> Self {
        Self {
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for MultiTurnMockProvider {
    async fn generate(
        &self,
        _request: &GenerateRequest,
    ) -> Result<GenerateResponse, ProviderError> {
        let mut count = self.call_count.lock().await;
        *count += 1;

        let response = match *count {
            1 => {
                // First call: return tool calls
                let tool_call = ToolCall::new(
                    "call_123".to_string(),
                    "echo".to_string(),
                    serde_json::json!({"message": "Hello from tool"}),
                );

                GenerateResponseBuilder::default()
                    .outputs(vec![Output::ToolCalls(vec![tool_call])])
                    .stop_reason(StopReason::ToolUse)
                    .build()
                    .expect("Valid response")
            }
            _ => {
                // Subsequent calls: return final text
                GenerateResponseBuilder::default()
                    .outputs(vec![Output::Text(
                        "I've executed the tool and here's the result.".to_string(),
                    )])
                    .stop_reason(StopReason::EndTurn)
                    .build()
                    .expect("Valid response")
            }
        };

        Ok(response)
    }

    fn provider_name(&self) -> &str {
        "multi-turn-mock"
    }

    fn default_model(&self) -> &str {
        "test-model"
    }

    fn supports_tools(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_end_to_end_sampling_with_tool_use() {
    // Setup
    let provider = Arc::new(MultiTurnMockProvider::new());
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry.clone()));

    // Create coordinator
    let coordinator = SamplingCoordinator::new(sampler, tool_registry);

    // Generate narrative (this will trigger the full sampling loop)
    let result = coordinator
        .generate_narrative("Create a test narrative".to_string())
        .await;

    // Verify success - coordinator returns a PartialNarrative
    // (even if placeholder, the flow should work)
    assert!(result.is_ok(), "Narrative generation should succeed");
}

#[tokio::test]
async fn test_full_sampling_loop_with_tool_execution() {
    // Setup
    let provider = Arc::new(MultiTurnMockProvider::new());
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry));

    let mut session = ConversationSession::new("You are a helpful assistant");
    session.add_turn(ConversationTurn::UserMessage {
        content: "Please use the echo tool".to_string(),
        attachments: None,
    });

    let tools = vec![];
    let result = sampler.sample(&mut session, &tools).await;

    assert!(result.is_ok());

    // Verify session contains the expected turns:
    // 1. User message (initial)
    // 2. Assistant tool calls
    // 3. Tool results
    // 4. Assistant final response
    assert!(session.turn_count() >= 3, "Should have at least 3 turns");

    // Check that we have tool calls turn
    let has_tool_calls = session
        .turns
        .iter()
        .any(|turn| matches!(turn, ConversationTurn::AssistantToolCalls { .. }));
    assert!(has_tool_calls, "Should have tool calls turn");

    // Check that we have tool results turn
    let has_tool_results = session
        .turns
        .iter()
        .any(|turn| matches!(turn, ConversationTurn::ToolResults { .. }));
    assert!(has_tool_results, "Should have tool results turn");
}

#[tokio::test]
async fn test_session_state_tracking() {
    let provider = Arc::new(MultiTurnMockProvider::new());
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry));

    let mut session = ConversationSession::new("System prompt");

    // Initial state
    assert_eq!(session.turn_count(), 0);
    assert!(session.is_active());

    // Add user message
    session.add_turn(ConversationTurn::UserMessage {
        content: "Hello".to_string(),
        attachments: None,
    });
    assert_eq!(session.turn_count(), 1);

    // Run sampling - will add more turns
    let tools = vec![];
    let _ = sampler.sample(&mut session, &tools).await;

    // Should have added turns
    assert!(session.turn_count() > 1, "Sampling should add turns");
}

/// Mock provider that returns only text (no tools).
struct SimpleTextProvider;

#[async_trait::async_trait]
impl LlmProvider for SimpleTextProvider {
    async fn generate(
        &self,
        _request: &GenerateRequest,
    ) -> Result<GenerateResponse, ProviderError> {
        Ok(GenerateResponseBuilder::default()
            .outputs(vec![Output::Text("Simple response".to_string())])
            .stop_reason(StopReason::EndTurn)
            .build()
            .expect("Valid response"))
    }

    fn provider_name(&self) -> &str {
        "simple-text"
    }

    fn default_model(&self) -> &str {
        "test"
    }
}

#[tokio::test]
async fn test_simple_conversation_without_tools() {
    let provider = Arc::new(SimpleTextProvider);
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry));

    let mut session = ConversationSession::new("You are helpful");
    session.add_turn(ConversationTurn::UserMessage {
        content: "Hello".to_string(),
        attachments: None,
    });

    let tools = vec![];
    let result = sampler.sample(&mut session, &tools).await;

    assert!(result.is_ok());
    let sampling_result = result.unwrap();

    match sampling_result {
        botticelli_mcp::SamplingResult::Completed { final_response } => {
            assert_eq!(final_response, "Simple response");
        }
    }

    // Should have exactly 2 turns: user + assistant
    assert_eq!(session.turn_count(), 2);
}

#[tokio::test]
async fn test_conversation_with_multiple_messages() {
    let provider = Arc::new(SimpleTextProvider);
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry));

    let mut session = ConversationSession::new("System");

    // Simulate a multi-turn conversation
    session.add_turn(ConversationTurn::UserMessage {
        content: "First question".to_string(),
        attachments: None,
    });

    session.add_turn(ConversationTurn::AssistantMessage {
        content: "First answer".to_string(),
    });

    session.add_turn(ConversationTurn::UserMessage {
        content: "Second question".to_string(),
        attachments: None,
    });

    let initial_turn_count = session.turn_count();
    assert_eq!(initial_turn_count, 3);

    // Run sampling - should add one more assistant response
    let tools = vec![];
    let result = sampler.sample(&mut session, &tools).await;

    assert!(result.is_ok());
    assert_eq!(session.turn_count(), initial_turn_count + 1);
}

/// Mock provider that simulates an error.
struct ErrorProvider;

#[async_trait::async_trait]
impl LlmProvider for ErrorProvider {
    async fn generate(
        &self,
        _request: &GenerateRequest,
    ) -> Result<GenerateResponse, ProviderError> {
        Err(ProviderError::new(
            "error-provider",
            ProviderErrorKind::ApiError("Simulated API error".to_string()),
        ))
    }

    fn provider_name(&self) -> &str {
        "error-provider"
    }

    fn default_model(&self) -> &str {
        "test"
    }
}

#[tokio::test]
async fn test_provider_error_handling() {
    let provider = Arc::new(ErrorProvider);
    let tool_registry = Arc::new(ToolRegistry::default());
    let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry));

    let mut session = ConversationSession::new("System");
    session.add_turn(ConversationTurn::UserMessage {
        content: "Test".to_string(),
        attachments: None,
    });

    let tools = vec![];
    let result = sampler.sample(&mut session, &tools).await;

    // Should propagate the error
    assert!(result.is_err());

    // Session should not have been modified
    assert_eq!(session.turn_count(), 1);
}
