use botticelli_mcp_client::{
    FinishReason, GenerationConfig, GenerationResponse, LlmAdapter, LlmToolCall, McpClientResult,
    Message, MessageRole, Orchestrator, ToolHandler, ToolRegistry, TokenUsage,
};
use async_trait::async_trait;
use pmcp::{Content, ToolInfo};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// Mock LLM adapter for testing orchestrator without real API calls.
struct MockLlmAdapter {
    model: String,
    responses: Mutex<Vec<GenerationResponse>>,
}

impl MockLlmAdapter {
    fn new(model: impl Into<String>, responses: Vec<GenerationResponse>) -> Self {
        Self {
            model: model.into(),
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
            panic!("No more mock responses available");
        }
        Ok(responses.remove(0))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn max_context_tokens(&self) -> u32 {
        4096
    }
}

/// Simple echo tool handler for testing
struct EchoToolHandler;

#[async_trait]
impl ToolHandler for EchoToolHandler {
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Ok(vec![Content::Text {
            text: format!("Echo: {}", message),
        }])
    }

    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "echo",
            Some("Echoes the input".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string"}
                },
                "required": ["message"]
            }),
        )
    }
}

/// Failing tool handler for testing error handling
struct FailingToolHandler;

#[async_trait]
impl ToolHandler for FailingToolHandler {
    async fn execute(&self, _args: Value) -> McpClientResult<Vec<Content>> {
        Err(botticelli_mcp_client::McpClientError::new(
            botticelli_mcp_client::McpClientErrorKind::ToolExecutionFailed(
                "Intentional failure".to_string(),
            ),
        ))
    }

    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "failing_tool",
            Some("Always fails".to_string()),
            json!({"type": "object"}),
        )
    }
}

/// No-op tool handler for testing
struct NoopToolHandler;

#[async_trait]
impl ToolHandler for NoopToolHandler {
    async fn execute(&self, _args: Value) -> McpClientResult<Vec<Content>> {
        Ok(vec![Content::Text {
            text: "Done".to_string(),
        }])
    }

    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "noop",
            Some("Does nothing".to_string()),
            json!({"type": "object"}),
        )
    }
}

/// Integration test: LLM → Tool Execution boundary
///
/// Tests that orchestrator correctly:
/// 1. Receives ToolCalls from LLM
/// 2. Executes tools via registry
/// 3. Formats results for LLM
#[tokio::test]
async fn test_orchestrator_executes_tools() {
    // Setup: Register echo tool
    let mut registry = ToolRegistry::new();
    registry
        .register("echo".to_string(), Arc::new(EchoToolHandler))
        .expect("Failed to register tool");
    let registry = Arc::new(registry);

    // Mock LLM that requests tool call, then completes
    let mock_responses = vec![
        // First response: Request tool call
        GenerationResponse::new(
            Message::new(
                MessageRole::Assistant,
                String::new(),
                vec![LlmToolCall::new(
                    "call_1".to_string(),
                    "echo".to_string(),
                    json!({"message": "Hello"}),
                )],
                Vec::new(),
            ),
            TokenUsage::new(10, 20, 30),
            FinishReason::ToolCalls,
        ),
        // Second response: Complete after tool result
        GenerationResponse::new(
            Message::new(
                MessageRole::Assistant,
                "I echoed your message.".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            TokenUsage::new(15, 25, 40),
            FinishReason::Stop,
        ),
    ];

    let adapter = Arc::new(MockLlmAdapter::new("mock-model", mock_responses));
    let orchestrator = Orchestrator::new(registry, adapter, 5);

    // Execute
    let result = orchestrator
        .execute(vec![Message::new(
            MessageRole::User,
            "Echo hello".to_string(),
            Vec::new(),
            Vec::new(),
        )])
        .await
        .expect("Orchestration failed");

    assert_eq!(result, "I echoed your message.");
}

/// Integration test: Tool Execution → Tool Result boundary
///
/// Tests that tool execution errors are properly handled and formatted.
#[tokio::test]
async fn test_orchestrator_handles_tool_errors() {
    let mut registry = ToolRegistry::new();
    registry
        .register("failing_tool".to_string(), Arc::new(FailingToolHandler))
        .expect("Failed to register tool");
    let registry = Arc::new(registry);

    let mock_responses = vec![
        // Request tool call
        GenerationResponse::new(
            Message::new(
                MessageRole::Assistant,
                String::new(),
                vec![LlmToolCall::new(
                    "call_1".to_string(),
                    "failing_tool".to_string(),
                    json!({}),
                )],
                Vec::new(),
            ),
            TokenUsage::new(10, 20, 30),
            FinishReason::ToolCalls,
        ),
        // Complete after receiving error
        GenerationResponse::new(
            Message::new(
                MessageRole::Assistant,
                "The tool failed, but I handled it.".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            TokenUsage::new(15, 25, 40),
            FinishReason::Stop,
        ),
    ];

    let adapter = Arc::new(MockLlmAdapter::new("mock-model", mock_responses));
    let orchestrator = Orchestrator::new(registry, adapter, 5);

    let result = orchestrator
        .execute(vec![Message::new(
            MessageRole::User,
            "Try the failing tool".to_string(),
            Vec::new(),
            Vec::new(),
        )])
        .await
        .expect("Orchestration should handle tool errors");

    assert_eq!(result, "The tool failed, but I handled it.");
}

/// Integration test: Max iterations boundary
///
/// Tests that orchestrator stops after max iterations to prevent infinite loops.
#[tokio::test]
async fn test_orchestrator_respects_max_iterations() {
    let mut registry = ToolRegistry::new();
    registry
        .register("noop".to_string(), Arc::new(NoopToolHandler))
        .expect("Failed to register tool");
    let registry = Arc::new(registry);

    // Mock LLM that always requests tool calls (infinite loop scenario)
    let mock_responses = vec![
        GenerationResponse::new(
            Message::new(
                MessageRole::Assistant,
                String::new(),
                vec![LlmToolCall::new(
                    "call_1".to_string(),
                    "noop".to_string(),
                    json!({}),
                )],
                Vec::new(),
            ),
            TokenUsage::new(10, 20, 30),
            FinishReason::ToolCalls,
        );
        10 // More responses than max_iterations
    ];

    let adapter = Arc::new(MockLlmAdapter::new("mock-model", mock_responses));
    let max_iterations = 3;
    let orchestrator = Orchestrator::new(registry, adapter, max_iterations);

    let result = orchestrator
        .execute(vec![Message::new(
            MessageRole::User,
            "Keep calling tools".to_string(),
            Vec::new(),
            Vec::new(),
        )])
        .await;

    assert!(result.is_err(), "Should fail after max iterations");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Maximum iterations exceeded"),
        "Error should mention max iterations"
    );
}
