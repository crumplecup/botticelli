#![cfg(feature = "gemini")]

// Tests using MockGeminiClient.
//
// These tests validate GeminiClient behavior without making real API calls,
// using a mock implementation for fast, deterministic testing.

mod test_utils;

use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_core::MessageBuilder;
use botticelli_error::GeminiErrorKind;
use botticelli_interface::BotticelliDriver;
use test_utils::{MockGeminiClient, MockResponse};

#[tokio::test]
async fn test_mock_basic_generate() -> anyhow::Result<()> {
    let mock = MockGeminiClient::new_success("Hello from mock!");

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Say hello".to_string())])
        .build()?;
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build().unwrap();

    let response = mock.generate(&request).await?;

    assert!(!response.outputs.is_empty());
    assert_eq!(mock.call_count(), 1);
    Ok(())
}

#[tokio::test]
async fn test_mock_multiple_requests() -> anyhow::Result<()> {
    let mock = MockGeminiClient::new_success("Response");

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build().unwrap();

    // First request
    let _response1 = mock.generate(&request).await?;
    assert_eq!(mock.call_count(), 1);

    // Second request
    let _response2 = mock.generate(&request).await?;
    assert_eq!(mock.call_count(), 2);

    // Third request
    let _response3 = mock.generate(&request).await?;
    assert_eq!(mock.call_count(), 3);
    Ok(())
}

#[tokio::test]
async fn test_mock_error_503() -> anyhow::Result<()> {
    let mock = MockGeminiClient::new_error(GeminiErrorKind::HttpError {
        status_code: 503,
        message: "Model is overloaded".to_string(),
    });

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build().unwrap();

    let result = mock.generate(&request).await;
    assert!(result.is_err());
    assert_eq!(mock.call_count(), 1);
    Ok(())
}

#[tokio::test]
async fn test_mock_retry_behavior() -> anyhow::Result<()> {
    // Mock fails twice with 503, then succeeds
    let mock = MockGeminiClient::new_fail_then_succeed(
        2,
        GeminiErrorKind::HttpError {
            status_code: 503,
            message: "Service unavailable".to_string(),
        },
        "Success after retries",
    );

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build().unwrap();

    // First two calls fail
    assert!(mock.generate(&request).await.is_err());
    assert!(mock.generate(&request).await.is_err());

    // Third call succeeds
    let response = mock.generate(&request).await?;
    assert!(!response.outputs.is_empty());
    assert_eq!(mock.call_count(), 3);
    Ok(())
}

#[tokio::test]
async fn test_mock_rate_limit_error() -> anyhow::Result<()> {
    let mock = MockGeminiClient::new_error(GeminiErrorKind::HttpError {
        status_code: 429,
        message: "Rate limit exceeded".to_string(),
    });

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build().unwrap();

    let result = mock.generate(&request).await;
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn test_mock_sequence_mixed_responses() -> anyhow::Result<()> {
    let mock = MockGeminiClient::new_sequence(vec![
        MockResponse::Success("First response".to_string()),
        MockResponse::Error(GeminiErrorKind::HttpError {
            status_code: 503,
            message: "Temporary error".to_string(),
        }),
        MockResponse::Success("Third response".to_string()),
    ]);

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build().unwrap();

    // First succeeds
    let response1 = mock.generate(&request).await?;
    assert!(!response1.outputs.is_empty());

    // Second fails
    assert!(mock.generate(&request).await.is_err());

    // Third succeeds
    let response3 = mock.generate(&request).await?;
    assert!(!response3.outputs.is_empty());

    assert_eq!(mock.call_count(), 3);
    Ok(())
}

#[tokio::test]
async fn test_mock_provider_name() {
    let mock = MockGeminiClient::new_success("test");
    assert_eq!(mock.provider_name(), "mock-gemini");
}

#[tokio::test]
async fn test_mock_model_name() {
    let mock = MockGeminiClient::new_success("test");
    assert_eq!(mock.model_name(), "mock-gemini");
}

#[tokio::test]
async fn test_mock_different_error_types() -> anyhow::Result<()> {
    // Test authentication error (401)
    let mock_auth = MockGeminiClient::new_error(GeminiErrorKind::HttpError {
        status_code: 401,
        message: "Invalid API key".to_string(),
    });

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build().unwrap();

    assert!(mock_auth.generate(&request).await.is_err());

    // Test bad request error (400)
    let mock_bad_request = MockGeminiClient::new_error(GeminiErrorKind::HttpError {
        status_code: 400,
        message: "Invalid request format".to_string(),
    });

    assert!(mock_bad_request.generate(&request).await.is_err());
    Ok(())
}
