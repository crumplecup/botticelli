//! Integration test for Groq backend LLM tool.

use botticelli_core::{GenerateRequest, GenerateRequestBuilder, Input, Message, MessageBuilder, Role};
use botticelli_mcp::{BotticelliServer, GenerateWithBackendRequest, GenerateWithBackendRequestBuilder, GroqBackendFactory};
use botticelli_models::GroqDriver;
use std::sync::Arc;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
#[cfg(feature = "groq")]
async fn test_groq_backend_simple_generation() {
    let _  = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Check API key is available
    std::env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set for API tests");

    // Create message
    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Say 'test' and nothing else.".to_string())])
        .build()
        .expect("Valid message");

    // Create request with backend specification
    let request = GenerateWithBackendRequestBuilder::default()
        .backend("groq".to_string())
        .model("llama-3.1-8b-instant".to_string())
        .request(
            GenerateRequestBuilder::default()
                .messages(vec![message])
                .max_tokens(Some(10))
                .build()
                .expect("Valid request"),
        )
        .build()
        .expect("Valid backend request");

    // Create factory and tool
    let factory = Arc::new(GroqBackendFactory);
    let tool = botticelli_mcp::GenerateWithBackendTool::new(factory);

    // Execute
    let response = tool
        .execute(request)
        .await
        .expect("Generation should succeed");

    // Validate response
    assert_eq!(response.provider(), "groq");
    assert_eq!(response.model(), "llama-3.1-8b-instant");
    assert!(!response.response().text().is_empty());

    println!("Generated text: {}", response.response().text());
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
#[cfg(feature = "groq")]
async fn test_groq_backend_via_rmcp_server() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Check API key
    std::env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set for API tests");

    // Create server with Groq driver
    let driver = GroqDriver::new("llama-3.1-8b-instant".to_string())
        .expect("Driver should initialize");

    let server = BotticelliServer::builder()
        .groq(Arc::new(driver))
        .build();

    // This test verifies the server is configured
    // Actual tool invocation would require rmcp transport setup
    // which is beyond the scope of this integration test
    drop(server);
}
