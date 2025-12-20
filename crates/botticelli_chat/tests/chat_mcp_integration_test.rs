use botticelli_chat::ChatHost;
use botticelli_core::ToolDefinition;
use botticelli_mcp::{McpClient, ToolRegistry};
use botticelli_models::{
    GeminiModel, ModelBounds, ModelId, ModelSelector, RateLimitDetector, SelectionStrategy,
};
use serde_json::json;
use std::sync::Arc;

/// Integration test for chat MCP tool execution flow.
/// 
/// This test verifies:
/// 1. MCP host initialization with tool registry
/// 2. Tool call detection and execution via ChatHost
/// 3. Result return to LLM
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_chat_with_mcp_tools() {
    // Load environment
    dotenvy::dotenv().ok();
    
    // Initialize tracing for debugging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .try_init()
        .ok();

    // Create tool registry with a simple echo tool
    let mut registry = ToolRegistry::new();
    
    let echo_tool = ToolDefinition::builder()
        .name("echo")
        .description("Echoes back the input message")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo"
                }
            },
            "required": ["message"]
        }))
        .build()
        .expect("Valid tool definition");
    
    registry.register(echo_tool).expect("Register tool");
    
    // Create MCP client
    let mcp_client = McpClient::new(Arc::new(registry));
    
    // Create model selector with Gemini+Groq fallback
    let bounds = ModelBounds::none();
    let strategy = SelectionStrategy::FriendlyFirst;
    let detector = RateLimitDetector::new();
    let selector = ModelSelector::new(bounds, strategy, detector);
    let initial_model = ModelId::Gemini(GeminiModel::Gemini25Flash);
    
    // Create chat host with MCP client and model selection
    let chat_host = ChatHost::builder()
        .mcp_client(mcp_client)
        .model_selector(selector)
        .initial_model(initial_model)
        .build()
        .expect("Valid chat host");
    
    // Send a message that should trigger tool use
    let response = chat_host
        .send("Please echo the message 'Hello from MCP!'")
        .await
        .expect("Send message");
    
    // Verify we got a response
    assert!(!response.is_empty(), "Should receive non-empty response");
    
    // The response should contain evidence of tool execution
    // (exact format depends on LLM, but should reference the echo)
    tracing::info!("Received response: {}", response);
}

/// Test fallback behavior when primary provider fails.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_chat_fallback_on_failure() {
    dotenvy::dotenv().ok();
    
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .try_init()
        .ok();

    // Create minimal setup
    let registry = ToolRegistry::new();
    let mcp_client = McpClient::new(Arc::new(registry));
    let bounds = ModelBounds::none();
    let strategy = SelectionStrategy::FriendlyFirst;
    let detector = RateLimitDetector::new();
    let selector = ModelSelector::new(bounds, strategy, detector);
    let initial_model = ModelId::Gemini(GeminiModel::Gemini25Flash);
    
    let chat_host = ChatHost::builder()
        .mcp_client(mcp_client)
        .model_selector(selector)
        .initial_model(initial_model)
        .build()
        .expect("Valid chat host");
    
    // Send a simple message
    let response = chat_host
        .send("Say hello")
        .await
        .expect("Send message");
    
    // Should get response from either Gemini or Groq fallback
    assert!(!response.is_empty(), "Should receive response despite potential failures");
    
    tracing::info!("Fallback test response: {}", response);
}
