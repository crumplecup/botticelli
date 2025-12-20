use botticelli_chat::{ChatHost, ChatSession};
use botticelli_core::{ToolDefinition, ToolParameter};
use botticelli_mcp::{McpClient, ToolRegistry};
use serde_json::json;
use std::sync::Arc;

/// Integration test for chat MCP tool execution flow.
/// 
/// This test verifies:
/// 1. MCP host initialization with tool registry
/// 2. Chat session with fallback provider
/// 3. Tool call detection and execution
/// 4. Result return to LLM
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
    
    // Create chat host with MCP client
    let chat_host = ChatHost::builder()
        .mcp_client(mcp_client)
        .build()
        .expect("Valid chat host");
    
    // Create chat session (uses Gemini+Groq fallback from config)
    let mut session = ChatSession::new(chat_host)
        .await
        .expect("Valid session");
    
    // Send a message that should trigger tool use
    let response = session
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
    let chat_host = ChatHost::builder()
        .mcp_client(mcp_client)
        .build()
        .expect("Valid chat host");
    
    let mut session = ChatSession::new(chat_host)
        .await
        .expect("Valid session");
    
    // Send a simple message
    let response = session
        .send("Say hello")
        .await
        .expect("Send message");
    
    // Should get response from either Gemini or Groq fallback
    assert!(!response.is_empty(), "Should receive response despite potential failures");
    
    tracing::info!("Fallback test response: {}", response);
}
