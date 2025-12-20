use botticelli_chat::{ChatAppConfig, ServiceContainer};
use botticelli_core::{MessageBuilder, Role};
use botticelli_models::ModelId;

/// Test that LLM provider can be initialized and used.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_llm_provider_initialization() {
    // Create config with default settings
    let config = ChatAppConfig::default();
    let services = ServiceContainer::new(config);

    // Initialize provider (lazy)
    let provider = services
        .llm_provider()
        .await
        .expect("Should initialize provider");

    // Verify we got a provider
    assert!(!std::ptr::eq(provider.as_ref(), std::ptr::null()));
}

/// Test that we can create clients for different models.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_create_client_for_model() {
    let config = ChatAppConfig::default();
    let services = ServiceContainer::new(config);

    // Create Gemini client
    let model = ModelId::Gemini(botticelli_models::GeminiModel::Gemini25Flash);
    let client = services
        .create_client_for_model(model)
        .expect("Should create Gemini client");

    // Verify we got a client
    assert!(!std::ptr::eq(client.as_ref(), std::ptr::null()));
}

/// Test tool-calling client creation.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_tool_calling_client() {
    let config = ChatAppConfig::default();
    let services = ServiceContainer::new(config);

    // Get tool-calling provider
    let result = services.llm_provider_with_tools().await;

    // Should succeed for Gemini (default)
    assert!(
        result.is_ok(),
        "Should create tool-calling client for Gemini"
    );
}

/// Test basic generation with initialized provider.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_basic_generation() {
    let config = ChatAppConfig::default();
    let services = ServiceContainer::new(config);

    let provider = services
        .llm_provider()
        .await
        .expect("Should initialize provider");

    // Create simple request
    let message = MessageBuilder::default()
        .role(Role::User)
        .content("Say hello".to_string())
        .build()
        .expect("Valid message");

    let request = botticelli_core::GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build();

    // Generate response
    let result = provider.generate(&request).await;

    // Should succeed
    assert!(result.is_ok(), "Generation should succeed");
    let response = result.unwrap();
    assert!(!response.content().is_empty(), "Should return content");
}
