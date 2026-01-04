//! Tests for Ollama client implementation.
//!
//! These tests require Ollama to be running locally with the llama2 model installed.
//! Install Ollama: https://ollama.ai/download
//! Pull model: ollama pull llama2
//!
//! Run with: cargo test --package botticelli_models --features ollama

#![cfg(feature = "ollama")]

use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_error::OllamaErrorKind;
use botticelli_interface::BotticelliDriver;
use botticelli_models::OllamaClient;

#[tokio::test]
#[ignore] // Requires Ollama running locally
async fn test_ollama_basic_generation() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("llama2")?;

    // Validate server and model
    client.validate().await?;

    let messages = vec![
        Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("Say hello".to_string())])
            .build()?,
    ];

    let request = GenerateRequest::builder().messages(messages).build()?;

    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response: {:?}", response.outputs());
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_ollama_model_validation() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("nonexistent_model")?;

    // Should fail - model doesn't exist
    let result = client.validate().await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e.kind(), OllamaErrorKind::ModelNotFound(_)));
    }
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_ollama_server_not_running() -> Result<(), Box<dyn std::error::Error>> {
    // Use non-standard port where Ollama is unlikely to be running
    let client = OllamaClient::new_with_url("llama2", "http://localhost:11435")?;

    let result = client.validate().await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e.kind(), OllamaErrorKind::ServerNotRunning(_)));
    }
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_ollama_multi_message_conversation() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("llama2")?;

    client.validate().await?;

    let messages = vec![
        Message::builder()
            .role(Role::System)
            .content(vec![Input::Text(
                "You are a helpful assistant.".to_string(),
            )])
            .build()?,
        Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("What is 2+2?".to_string())])
            .build()?,
    ];

    let request = GenerateRequest::builder().messages(messages).build()?;

    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response: {:?}", response.outputs());
    Ok(())
}
