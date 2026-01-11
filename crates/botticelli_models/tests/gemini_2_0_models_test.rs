#![cfg(feature = "gemini")]

mod helpers;


// Tests for Gemini 2.0 model compatibility.
//
// These tests validate that older Gemini 2.0 models work correctly
// via the Model::Custom() variant with proper "models/" prefix.

#[cfg(feature = "api")]
use botticelli_core::{GenerateRequest, Input, Message, Role};
#[cfg(feature = "api")]
use botticelli_interface::BotticelliDriver;
#[cfg(feature = "api")]
use botticelli_models::GeminiClient;

/// Test that Gemini 2.0 Flash works via Model::Custom with "models/" prefix.
#[tokio::test]
#[cfg(feature = "api")] // Requires GEMINI_API_KEY
async fn test_gemini_2_0_flash() -> anyhow::Result<()> {
    let client = GeminiClient::new()?;

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'ok'".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model("gemini-2.0-flash".to_string())
        .max_tokens(10u32)
        .build()?;

    let response = client.generate(&request).await?;
    assert!(!response.outputs().is_empty());
    tracing::info!("Test Gemini 2 0 Flash test passed");
    Ok(())
}

/// Test that Gemini 2.0 Flash Lite works via Model::Custom.
#[tokio::test]
#[cfg(feature = "api")] // Requires GEMINI_API_KEY
async fn test_gemini_2_0_flash_lite() -> anyhow::Result<()> {
    let client = GeminiClient::new()?;

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'ok'".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model("gemini-2.0-flash-lite".to_string())
        .max_tokens(10u32)
        .build()?;

    let response = client.generate(&request).await?;
    assert!(!response.outputs().is_empty());
    tracing::info!("Test Gemini 2 0 Flash Lite test passed");
    Ok(())
}

/// Test that multiple requests with mixed 2.0 and 2.5 models work correctly.
#[tokio::test]
#[cfg(feature = "api")] // Requires GEMINI_API_KEY
async fn test_mixed_2_0_and_2_5_models() -> anyhow::Result<()> {
    let client = GeminiClient::new()?;

    // Request 1: Use Gemini 2.0 Flash
    let message1 = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'one'".to_string())])
        .build()?;

    let request1 = GenerateRequest::builder()
        .messages(vec![message1])
        .model("gemini-2.0-flash".to_string())
        .max_tokens(10u32)
        .build()?;

    let response1 = client.generate(&request1).await?;
    assert!(!response1.outputs().is_empty());

    // Request 2: Use Gemini 2.5 Flash (should use enum variant)
    let message2 = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'two'".to_string())])
        .build()?;

    let request2 = GenerateRequest::builder()
        .messages(vec![message2])
        .model("gemini-2.5-flash".to_string())
        .max_tokens(10u32)
        .build()?;

    let response2 = client.generate(&request2).await?;
    assert!(!response2.outputs().is_empty());

    // Request 3: Use Gemini 2.0 Flash Lite
    let message3 = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'three'".to_string())])
        .build()?;

    let request3 = GenerateRequest::builder()
        .messages(vec![message3])
        .model("gemini-2.0-flash-lite".to_string())
        .max_tokens(10u32)
        .build()?;

    let response3 = client.generate(&request3).await?;
    assert!(!response3.outputs().is_empty());
    tracing::info!("Test Mixed 2 0 And 2 5 Models test passed");
    Ok(())
}

/// Test that explicit "models/" prefix is preserved.
#[tokio::test]
#[cfg(feature = "api")] // Requires GEMINI_API_KEY
async fn test_explicit_models_prefix() -> anyhow::Result<()> {
    let client = GeminiClient::new()?;

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Hello".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model("models/gemini-2.0-flash".to_string())
        .max_tokens(10u32)
        .build()?;

    let response = client.generate(&request).await?;
    assert!(!response.outputs().is_empty());
    tracing::info!("Test Explicit Models Prefix test passed");
    Ok(())
}
