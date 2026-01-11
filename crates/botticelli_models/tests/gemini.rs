#![cfg(feature = "gemini")]

mod helpers;


// Tests for the Gemini client implementation.

use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_error::{BotticelliError, GeminiError, GeminiErrorKind};
use botticelli_interface::{BotticelliDriver, Metadata, Vision};
use botticelli_models::GeminiClient;

// MessageBuilder trait is auto-imported via derive_builder

//
// ─── ERROR HANDLING TESTS ───────────────────────────────────────────────────────
//

#[test]
fn test_gemini_error_display() {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Test Gemini Error Display");

    let error = GeminiError::new(GeminiErrorKind::MissingApiKey);
    let display = format!("{}", error);
    assert!(display.contains("GEMINI_API_KEY environment variable not set"));
    assert!(display.contains("Gemini Error:"));
    assert!(display.contains("at line"));
}

#[test]
fn test_gemini_error_kind_display() {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Test Gemini Error Kind Display");

    let cases = vec![
        (
            GeminiErrorKind::MissingApiKey,
            "GEMINI_API_KEY environment variable not set",
        ),
        (
            GeminiErrorKind::InvalidServerMessage("request failed".to_string()),
            "Invalid server message: request failed",
        ),
        (
            GeminiErrorKind::MultimodalNotSupported,
            "Multimodal inputs not yet supported in simple Gemini wrapper",
        ),
        (
            GeminiErrorKind::UrlMediaNotSupported,
            "URL media sources not yet supported for Gemini",
        ),
        (
            GeminiErrorKind::Base64Decode("invalid base64".to_string()),
            "Base64 decode error: invalid base64",
        ),
    ];

    for (kind, expected) in cases {
        let display = format!("{}", kind);
        assert_eq!(display, expected, "Error kind display mismatch");
    }
}

#[test]
fn test_gemini_error_source_location_tracking() {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Test Gemini Error Source Location Tracking");

    let error = GeminiError::new(GeminiErrorKind::MissingApiKey);
    assert!(error.line > 0, "Error should capture line number");
    assert!(
        error.file.contains("gemini.rs"),
        "Error should capture file name"
    );
}

//
// ─── REQUEST BUILDING TESTS ─────────────────────────────────────────────────────
//

#[test]
fn test_simple_text_request_structure() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Test Simple Text Request Structure");

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Hello, world!".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(100u32)
        .temperature(0.7)
        .build()?;

    assert_eq!(request.messages().len(), 1);
    assert_eq!(*request.max_tokens(), Some(100));
    assert_eq!(*request.temperature(), Some(0.7));

    tracing::info!("Test Simple Text Request Structure test passed");
    Ok(())
}

#[test]
fn test_multi_message_request_structure() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Test Multi Message Request Structure");

    let message1 = Message::builder()
        .role(Role::System)
        .content(vec![Input::Text(
            "You are a helpful assistant.".to_string(),
        )])
        .build()?;

    let message2 = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("What is Rust?".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message1, message2])
        .build()?;

    assert_eq!(request.messages().len(), 2);
    assert_eq!(request.messages()[0].role(), &Role::System);
    assert_eq!(request.messages()[1].role(), &Role::User);

    tracing::info!("Test Multi Message Request Structure test passed");
    Ok(())
}

//
// ─── ERROR CONVERSION TESTS ─────────────────────────────────────────────────────
//

#[test]
fn test_gemini_error_to_botticelli_error_conversion() {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Test Gemini Error To Botticelli Error Conversion");

    let gemini_error = GeminiError::new(GeminiErrorKind::MissingApiKey);
    let botticelli_error: BotticelliError = gemini_error.into();

    let display = format!("{}", botticelli_error);
    assert!(display.contains("Botticelli Error:"));
    assert!(display.contains("Gemini Error:"));
}

#[test]
fn test_error_kind_comparison() {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Test Error Kind Comparison");

    // Test that errors can be compared
    let error1 = GeminiError::new(GeminiErrorKind::MissingApiKey);
    let error2 = GeminiError::new(GeminiErrorKind::MissingApiKey);

    // Both should have same kind
    assert!(format!("{}", error1.kind).contains("GEMINI_API_KEY"));
    assert!(format!("{}", error2.kind).contains("GEMINI_API_KEY"));
}

//
// ─── INTEGRATION TESTS ──────────────────────────────────────────────────────────
//

/// Integration test that requires a real API key and consumes tokens.
///
/// Run with: `cargo test --features gemini -- --ignored`
///
/// Note: This test requires the GEMINI_API_KEY environment variable to be set
/// with a valid API key before running.
#[test]
#[cfg(feature = "api")]
fn test_real_api_call() -> anyhow::Result<()> {
    let client = GeminiClient::new()?;

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'ok'".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(10u32)
        .temperature(0.0)
        .build()?;

    let rt = tokio::runtime::Runtime::new()?;
    let response = rt.block_on(async { client.generate(&request).await })?;

    assert!(
        !response.outputs().is_empty(),
        "Should have at least one output"
    );

    tracing::info!("Test Real Api Call test passed");
    Ok(())
}

/// Test that verifies client creation behavior and consumes tokens.
///
/// This test checks that client creation succeeds when GEMINI_API_KEY is set.
/// Run with: `cargo test --features gemini -- --ignored`
#[test]
#[cfg(feature = "api")]
fn test_client_creation() -> Result<(), botticelli_error::BotticelliError> {
    dotenvy::dotenv().ok();

    let client = GeminiClient::new()?;

    assert_eq!(client.provider_name(), "gemini");
    assert_eq!(client.model_name(), "gemini-2.5-flash");

    // Test metadata
    let metadata = client.metadata();
    // Just verify we can access metadata - it's an enum variant
    assert!(!format!("{}", metadata).is_empty());

    // Test vision trait
    assert_eq!(client.max_images_per_request(), 16);

    tracing::info!("Test Client Creation test passed");
    Ok(())
}
