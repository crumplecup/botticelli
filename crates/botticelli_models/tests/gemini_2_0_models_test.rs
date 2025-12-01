#![cfg(feature = "gemini")]

// Tests for Gemini 2.0 model compatibility.
//
// These tests validate that older Gemini 2.0 models work correctly
// via the Model::Custom() variant with proper "models/" prefix.

use botticelli_error::BotticelliResult;
use botticelli_interface::BotticelliDriver;
use botticelli_models::GeminiClient;

/// Test that Gemini 2.0 Flash works via Model::Custom with "models/" prefix.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_gemini_2_0_flash() -> BotticelliResult<()> {
    let client = GeminiClient::new()?;

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Say 'ok'".to_string())])
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model(Some("gemini-2.0-flash".to_string()))
        .max_tokens(Some(10))
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let response = client.generate(&request).await?;
    assert!(!response.outputs().is_empty());
    Ok(())
}

/// Test that Gemini 2.0 Flash Lite works via Model::Custom.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_gemini_2_0_flash_lite() -> BotticelliResult<()> {
    let client = GeminiClient::new()?;

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Say 'ok'".to_string())])
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model(Some("gemini-2.0-flash-lite".to_string()))
        .max_tokens(Some(10))
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let response = client.generate(&request).await?;
    assert!(!response.outputs().is_empty());
    Ok(())
}

/// Test that multiple requests with mixed 2.0 and 2.5 models work correctly.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_mixed_2_0_and_2_5_models() -> BotticelliResult<()> {
    let client = GeminiClient::new()?;

    // Request 1: Use Gemini 2.0 Flash
    let message1 = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Say 'one'".to_string())])
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let request1 = GenerateRequest::builder()
        .messages(vec![message1])
        .model(Some("gemini-2.0-flash".to_string()))
        .max_tokens(Some(10))
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let response1 = client.generate(&request1).await?;
    assert!(!response1.outputs.is_empty());

    // Request 2: Use Gemini 2.5 Flash (should use enum variant)
    let message2 = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Say 'two'".to_string())])
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let request2 = GenerateRequest::builder()
        .messages(vec![message2])
        .model(Some("gemini-2.5-flash".to_string()))
        .max_tokens(Some(10))
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let response2 = client.generate(&request2).await?;
    assert!(!response2.outputs().is_empty());

    // Request 3: Use Gemini 2.0 Flash Lite
    let message3 = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Say 'three'".to_string())])
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let request3 = GenerateRequest::builder()
        .messages(vec![message3])
        .model(Some("gemini-2.0-flash-lite".to_string()))
        .max_tokens(Some(10))
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let response3 = client.generate(&request3).await?;
    assert!(!response3.outputs.is_empty());
    Ok(())
}

/// Test that explicit "models/" prefix is preserved.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_explicit_models_prefix() -> BotticelliResult<()> {
    let client = GeminiClient::new()?;

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Hello".to_string())])
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model(Some("models/gemini-2.0-flash".to_string()))
        .max_tokens(Some(10))
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(botticelli_error::BotticelliErrorKind::Backend(
                botticelli_error::BackendError::new(format!("Builder error: {}", e)),
            ))
        })?;

    let response = client.generate(&request).await?;
    assert!(!response.outputs().is_empty());
    Ok(())
}
