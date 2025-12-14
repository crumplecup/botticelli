//! Test LlmProvider trait implementation for Anthropic.

#[cfg(feature = "anthropic")]
use botticelli_core::{GenerateRequest, LlmProvider, Message, Role, Input};
#[cfg(feature = "anthropic")]
use botticelli_models::AnthropicClient;

#[tokio::test]
#[cfg(feature = "anthropic")]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_anthropic_provider_trait() {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set for API tests");
    
    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");
    
    // Verify trait methods
    assert_eq!(client.provider_name(), "anthropic");
    assert_eq!(client.default_model(), "claude-3-5-sonnet-20241022");
    assert!(client.supports_tools());
    
    // Test generation via trait
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say hello in 3 words".to_string())])
        .build()
        .expect("Valid message");
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .build()
        .expect("Valid request");
    
    let response = client.generate(&request)
        .await
        .expect("Should generate response");
    
    // Verify we got a response with outputs
    assert!(!response.outputs().is_empty());
}
