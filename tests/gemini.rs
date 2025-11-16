//! Tests for the Gemini client implementation.

#![cfg(feature = "gemini")]

use boticelli::{
    BoticelliDriver, GeminiClient, GeminiError, GeminiErrorKind, GenerateRequest, Input, Message,
    Metadata, Role, Vision,
};

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
fn test_simple_text_request_structure() {
    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Hello, world!".to_string())],
        }],
        max_tokens: Some(100),
        temperature: Some(0.7),
        model: None,
    };

    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.max_tokens, Some(100));
    assert_eq!(request.temperature, Some(0.7));
}

#[test]
fn test_multi_message_request_structure() {
    let request = GenerateRequest {
        messages: vec![
            Message {
                role: Role::System,
                content: vec![Input::Text("You are a helpful assistant.".to_string())],
            },
            Message {
                role: Role::User,
                content: vec![Input::Text("What is Rust?".to_string())],
            },
        ],
        max_tokens: None,
        temperature: None,
        model: None,
    };

    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.messages[0].role, Role::System);
    assert_eq!(request.messages[1].role, Role::User);
}

//
// ─── ERROR CONVERSION TESTS ─────────────────────────────────────────────────────
//

#[test]
fn test_gemini_error_to_boticelli_error_conversion() {
    let gemini_error = GeminiError::new(GeminiErrorKind::MissingApiKey);
    let boticelli_error: boticelli::BoticelliError = gemini_error.into();

    let display = format!("{}", boticelli_error);
    assert!(display.contains("Boticelli Error:"));
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
/// This test is ignored unless the 'api' feature is enabled:
/// `cargo test --features gemini,api`
///
/// Note: This test requires the GEMINI_API_KEY environment variable to be set
/// with a valid API key before running.
#[test]
#[cfg_attr(not(feature = "api"), ignore)]
fn test_real_api_call() {
    // This test relies on GEMINI_API_KEY being set before the test runs
    // Do not manipulate environment variables in the test itself
    let client = match GeminiClient::new() {
        Ok(c) => c,
        Err(e) => {
            panic!("Failed to create client. Ensure GEMINI_API_KEY is set: {}", e);
        }
    };

    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'Hello, Boticelli!' and nothing else.".to_string())],
        }],
        max_tokens: Some(20),
        temperature: Some(0.0),
        model: None,
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async { client.generate(&request).await });

    assert!(result.is_ok(), "API call should succeed: {:?}", result.err());

    let response = result.unwrap();
    assert!(!response.outputs.is_empty(), "Should have at least one output");
}

/// Test that verifies client creation behavior and consumes tokens.
///
/// This test checks that client creation succeeds when GEMINI_API_KEY is set.
/// Run with: `cargo test --features gemini,api`
#[test]
#[cfg_attr(not(feature = "api"), ignore)]
fn test_client_creation() {
    // Assumes GEMINI_API_KEY is already set in environment
    let result = GeminiClient::new();

    match result {
        Ok(client) => {
            assert_eq!(client.provider_name(), "gemini");
            assert_eq!(client.model_name(), "gemini-2.5-flash");

            // Test metadata
            let metadata = client.metadata();
            assert_eq!(metadata.provider, "gemini");
            assert_eq!(metadata.max_input_tokens, 1_048_576);

            // Test vision trait
            assert_eq!(client.max_images_per_request(), 16);
        }
        Err(e) => {
            panic!("Failed to create client. Set GEMINI_API_KEY before running: {}", e);
        }
    }
}
