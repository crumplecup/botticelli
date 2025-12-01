//! Tests for Ollama client implementation.
//!
//! These tests require Ollama to be running locally with the llama2 model installed.
//! Install Ollama: https://ollama.ai/download
//! Pull model: ollama pull llama2
//!
//! Run with: cargo test --package botticelli_models --features ollama

#![cfg(feature = "ollama")]

use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_interface::BotticelliDriver;
use botticelli_models::{OllamaClient, OllamaErrorKind};

#[tokio::test]
#[ignore] // Requires Ollama running locally
async fn test_ollama_basic_generation() {
    let client = OllamaClient::new("llama2").expect("Failed to create client");

    // Validate server and model
    client
        .validate()
        .await
        .expect("Ollama server not available");

    let messages = vec![Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say hello".to_string())])
        .build()
        .expect("Valid message")];

    let request = GenerateRequest::builder()
        .messages(messages)
        .build()
        .expect("Valid request");

    let response = client.generate(&request).await.expect("Generation failed");

    assert!(!response.outputs().is_empty());
    println!("Response: {:?}", response.outputs());
}

#[tokio::test]
#[ignore]
async fn test_ollama_model_validation() {
    let client = OllamaClient::new("nonexistent_model").expect("Client creation should succeed");

    // Should fail - model doesn't exist
    let result = client.validate().await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e.kind, OllamaErrorKind::ModelNotFound(_)));
    }
}

#[tokio::test]
#[ignore]
async fn test_ollama_server_not_running() {
    // Use non-standard port where Ollama is unlikely to be running
    let client =
        OllamaClient::new_with_url("llama2", "http://localhost:11435").expect("Client creation");

    let result = client.validate().await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e.kind, OllamaErrorKind::ServerNotRunning(_)));
    }
}

#[tokio::test]
#[ignore]
async fn test_ollama_multi_message_conversation() {
    let client = OllamaClient::new("llama2").expect("Failed to create client");

    client
        .validate()
        .await
        .expect("Ollama server not available");

    let messages = vec![
        Message::builder()
            .role(Role::System)
            .content(vec![Input::Text(
                "You are a helpful assistant.".to_string(),
            )])
            .build()
            .expect("Valid message"),
        Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("What is 2+2?".to_string())])
            .build()
            .expect("Valid message"),
    ];

    let request = GenerateRequest::builder()
        .messages(messages)
        .build()
        .expect("Valid request");

    let response = client.generate(&request).await.expect("Generation failed");

    assert!(!response.outputs().is_empty());
    println!("Response: {:?}", response.outputs());
}
