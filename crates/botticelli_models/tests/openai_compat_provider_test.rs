//! Test LlmProvider trait implementation for OpenAI-compatible clients.

#[cfg(feature = "groq")]
use botticelli_core::{GenerateRequest, Input, Message, Role};
#[cfg(feature = "groq")]
use botticelli_interface::LlmProvider;
#[cfg(feature = "groq")]
use botticelli_models::OpenAICompatibleClient;

#[tokio::test]
#[cfg(feature = "groq")]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_groq_provider_trait() {
    let api_key = std::env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set for API tests");

    let client = OpenAICompatibleClient::new(
        api_key,
        "llama-3.3-70b-versatile".to_string(),
        "https://api.groq.com/openai/v1/chat/completions".to_string(),
        "groq",
    );

    // Verify trait methods
    assert_eq!(client.provider_name(), "groq");
    assert_eq!(client.default_model(), "llama-3.3-70b-versatile");
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

    let response = client
        .generate(&request)
        .await
        .expect("Should generate response");

    // Verify we got a response with outputs
    assert!(!response.outputs().is_empty());
}

#[tokio::test]
#[cfg(feature = "huggingface")]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_huggingface_provider_trait() {
    let api_key = std::env::var("HUGGINGFACE_API_KEY")
        .expect("HUGGINGFACE_API_KEY must be set for API tests");

    let client = OpenAICompatibleClient::new(
        api_key,
        "meta-llama/Llama-3.2-3B-Instruct".to_string(),
        "https://api-inference.huggingface.co/models/meta-llama/Llama-3.2-3B-Instruct/v1/chat/completions".to_string(),
        "huggingface",
    );

    // Verify trait methods
    assert_eq!(client.provider_name(), "huggingface");
    assert_eq!(client.default_model(), "meta-llama/Llama-3.2-3B-Instruct");
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

    let response = client
        .generate(&request)
        .await
        .expect("Should generate response");

    // Verify we got a response with outputs
    assert!(!response.outputs().is_empty());
}
