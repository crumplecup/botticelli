//! Integration test for Phase 4 orchestration layer.
//!
//! Tests the complete flow: ToolRegistry → Orchestrator → LLM Adapter → Tool Execution

use botticelli_mcp_client::{
    GenerationConfig, GenerationResponse, LlmAdapter, Message, MessageRole,
    Orchestrator, ToolHandler, ToolRegistry, LlmToolSchema,
};
use botticelli_mcp_client::{McpClientError, McpClientErrorKind, McpClientResult};
use async_trait::async_trait;
use pmcp::{Content, ToolInfo};
use serde_json::{json, Value};
use std::sync::Arc;

/// Mock tool that echoes input
struct EchoTool;

#[async_trait]
impl ToolHandler for EchoTool {
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("no message");

        Ok(vec![Content::Text {
            text: format!("Echo: {}", message),
        }])
    }

    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "echo",
            Some("Echoes the input message".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Message to echo"
                    }
                },
                "required": ["message"]
            }),
        )
    }
}

/// Mock LLM adapter that simulates tool calling
struct MockLlmAdapter {
    call_count: std::sync::Mutex<usize>,
}

impl MockLlmAdapter {
    fn new() -> Self {
        Self {
            call_count: std::sync::Mutex::new(0),
        }
    }
}

#[async_trait]
impl LlmAdapter for MockLlmAdapter {
    async fn generate(
        &self,
        messages: Vec<Message>,
        _tools: Vec<LlmToolSchema>,
        _config: GenerationConfig,
    ) -> McpClientResult<GenerationResponse> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;

        // First call: request tool use
        if *count == 1 {
            let tool_call = botticelli_mcp_client::LlmToolCall {
                id: "call_123".to_string(),
                name: "echo".to_string(),
                arguments: json!({"message": "Hello from test"}),
            };

            return Ok(GenerationResponse {
                message: Message {
                    role: MessageRole::Assistant,
                    content: String::new(),
                    tool_calls: vec![tool_call],
                    tool_results: Vec::new(),
                },
                usage: botticelli_mcp_client::TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                },
                finish_reason: botticelli_mcp_client::FinishReason::ToolCalls,
            });
        }

        // Second call: return final response after seeing tool result
        let last_message = messages.last().ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::LlmError("No messages".to_string()))
        })?;

        // Verify we received tool result
        if !last_message.tool_results.is_empty() {
            Ok(GenerationResponse {
                message: Message {
                    role: MessageRole::Assistant,
                    content: "Tool executed successfully!".to_string(),
                    tool_calls: Vec::new(),
                    tool_results: Vec::new(),
                },
                usage: botticelli_mcp_client::TokenUsage {
                    prompt_tokens: 20,
                    completion_tokens: 10,
                    total_tokens: 30,
                },
                finish_reason: botticelli_mcp_client::FinishReason::Stop,
            })
        } else {
            Err(McpClientError::new(McpClientErrorKind::LlmError(
                "Expected tool results".to_string(),
            )))
        }
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn max_context_tokens(&self) -> u32 {
        8192
    }
}

#[tokio::test]
async fn test_orchestrator_basic_flow() {
    // Set up registry with echo tool
    let mut registry = ToolRegistry::new();
    registry
        .register("echo".to_string(), Arc::new(EchoTool))
        .expect("Failed to register tool");

    // Create orchestrator with mock adapter
    let adapter = Arc::new(MockLlmAdapter::new());
    let orchestrator = Orchestrator::new(Arc::new(registry), adapter, 5);

    // Execute agentic loop
    let initial_messages = vec![Message {
        role: MessageRole::User,
        content: "Please echo a message".to_string(),
        tool_calls: Vec::new(),
        tool_results: Vec::new(),
    }];

    let result = orchestrator.execute(initial_messages).await;

    assert!(result.is_ok(), "Orchestrator failed: {:?}", result.err());
    let final_response = result.unwrap();
    assert_eq!(final_response, "Tool executed successfully!");
}

#[tokio::test]
async fn test_orchestrator_max_iterations() {
    // Create orchestrator with low max iterations
    let registry = ToolRegistry::new();
    let adapter = Arc::new(MockLlmAdapter::new());
    let orchestrator = Orchestrator::new(Arc::new(registry), adapter, 1);

    let initial_messages = vec![Message {
        role: MessageRole::User,
        content: "Test".to_string(),
        tool_calls: Vec::new(),
        tool_results: Vec::new(),
    }];

    // This will fail because mock always requests tools on first call
    let result = orchestrator.execute(initial_messages).await;

    assert!(result.is_err());
    match &result.unwrap_err().kind {
        McpClientErrorKind::MaxIterationsExceeded(_) => {}
        other => panic!("Expected MaxIterationsExceeded, got {:?}", other),
    }
}

#[tokio::test]
async fn test_tool_registry_operations() {
    let mut registry = ToolRegistry::new();

    // Register tool
    registry
        .register("echo".to_string(), Arc::new(EchoTool))
        .expect("Failed to register");

    assert!(registry.has_tool("echo"));
    assert_eq!(registry.tool_count(), 1);

    // List tools
    let tools = registry.list_tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");

    // Execute tool
    let result = registry
        .execute_tool("echo", json!({"message": "test"}))
        .await;

    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    match &content[0] {
        Content::Text { text } => assert_eq!(text, "Echo: test"),
        _ => panic!("Expected text content"),
    }
}
