#![cfg(feature = "gemini")]
mod test_utils;

// Tests for the Gemini client implementation.

use botticelli_error::{BotticelliError, GeminiError, GeminiErrorKind};
use botticelli_interface::{BotticelliDriver, Metadata, Vision};
use botticelli_models::GeminiClient;

// MessageBuilder trait is auto-imported via derive_builder

//
// ─── ERROR HANDLING TESTS ───────────────────────────────────────────────────────
//

#[test]
fn test_gemini_error_display() {
    let error = GeminiError::new(GeminiErrorKind::MissingApiKey);
    let display = format!("{}", error);
    assert!(display.contains("GEMINI_API_KEY environment variable not set"));
    assert!(display.contains("Gemini Error:"));
    assert!(display.contains("at line"));
}

#[test]
fn test_gemini_error_kind_display() {
    let cases = vec![
        (
            GeminiErrorKind::MissingApiKey,
            "GEMINI_API_KEY environment variable not set",
        ),
        (
            GeminiErrorKind::ClientCreation("test error".to_string()),
            "Failed to create Gemini client: test error",
        ),
        (
            GeminiErrorKind::ApiRequest("request failed".to_string()),
            "Gemini API request failed: request failed",
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
    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Hello, world!".to_string())])
        .build()
        .expect("Valid message");

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(100))
        .temperature(Some(0.7))
        .build()
        .expect("Valid request");

    assert_eq!(request.messages().len(), 1);
    assert_eq!(*request.max_tokens(), Some(100));
    assert_eq!(*request.temperature(), Some(0.7));

    Ok(())
}

#[test]
fn test_multi_message_request_structure() {
    let message1 = MessageBuilder::default()
        .role(Role::System)
        .content(vec![Input::Text(
            "You are a helpful assistant.".to_string(),
        )])
        .build()
        .expect("Failed to build message");

    let message2 = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("What is Rust?".to_string())])
        .build()
        .expect("Failed to build message");

    let request = GenerateRequest::builder()
        .messages(vec![message1, message2])
        .build()
        .expect("Failed to build request");

    assert_eq!(request.messages().len(), 2);
    assert_eq!(request.messages()[0].role(), &Role::System);
    assert_eq!(request.messages()[1].role(), &Role::User);
}

//
// ─── ERROR CONVERSION TESTS ─────────────────────────────────────────────────────
//

#[test]
fn test_gemini_error_to_botticelli_error_conversion() {
    let gemini_error = GeminiError::new(GeminiErrorKind::MissingApiKey);
    let botticelli_error: BotticelliError = gemini_error.into();

    let display = format!("{}", botticelli_error);
    assert!(display.contains("Botticelli Error:"));
    assert!(display.contains("Gemini Error:"));
}

#[test]
fn test_error_kind_equality() {
    let error1 = GeminiErrorKind::MissingApiKey;
    let error2 = GeminiErrorKind::MissingApiKey;
    let error3 = GeminiErrorKind::MultimodalNotSupported;

    assert_eq!(error1, error2);
    assert_ne!(error1, error3);
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
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
fn test_real_api_call() {
    // This test relies on GEMINI_API_KEY being set before the test runs
    // Do not manipulate environment variables in the test itself
    let client = match GeminiClient::new() {
        Ok(c) => c,
        Err(e) => {
            panic!(
                "Failed to create client. Ensure GEMINI_API_KEY is set: {}",
                e
            );
        }
    };

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Say 'ok'".to_string())])
        .build()
        .expect("Failed to build message");

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .temperature(Some(0.0))
        .build()
        .expect("Failed to build request");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async { client.generate(&request).await });

    assert!(
        result.is_ok(),
        "API call should succeed: {:?}",
        result.err()
    );

    let response = result.unwrap();
    assert!(
        !response.outputs.is_empty(),
        "Should have at least one output"
    );
}

/// Test that verifies client creation behavior and consumes tokens.
///
/// This test checks that client creation succeeds when GEMINI_API_KEY is set.
/// Run with: `cargo test --features gemini -- --ignored`
#[test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
fn test_client_creation() {
    // Assumes GEMINI_API_KEY is already set in environment
    let result = GeminiClient::new();

    match result {
        Ok(client) => {
            assert_eq!(client.provider_name(), "gemini");
            assert_eq!(client.model_name(), "gemini-2.0-flash-lite");

            // Test metadata
            let metadata = client.metadata();
            assert_eq!(metadata.provider, "gemini");
            assert_eq!(metadata.max_input_tokens, 1_048_576);

            // Test vision trait
            assert_eq!(client.max_images_per_request(), 16);
        }
        Err(e) => {
            panic!(
                "Failed to create client. Set GEMINI_API_KEY before running: {}",
                e
            );
        }
    }
}
