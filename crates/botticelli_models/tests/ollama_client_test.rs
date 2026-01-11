//! Tests for Ollama client implementation.
//!
//! These tests require Ollama to be running locally with the llama2 model installed.
//! Install Ollama: https://ollama.ai/download
//! Pull model: ollama pull llama2
//!
//! Run with: cargo test --package botticelli_models --features ollama


#![cfg(feature = "ollama")]

mod helpers;

use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_error::OllamaErrorKind;
use botticelli_interface::BotticelliDriver;
use botticelli_models::OllamaClient;

#[tokio::test]
#[ignore] // Requires Ollama running locally
async fn test_ollama_basic_generation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Ollama basic generation");
    
    let client = OllamaClient::new("llama2")?;
    tracing::debug!(model = "llama2", "Created OllamaClient");

    // Validate server and model
    tracing::debug!("Validating Ollama server and model");
    client.validate().await?;

    let messages = vec![
        Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("Say hello".to_string())])
            .build()?,
    ];

    let request = GenerateRequest::builder().messages(messages).build()?;

    tracing::debug!("Sending generation request");
    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response: {:?}", response.outputs());
    tracing::debug!(output_count = response.outputs().len(), "Received response");
    
    tracing::info!("Ollama basic generation test passed");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_ollama_model_validation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Ollama model validation (nonexistent model)");
    
    let client = OllamaClient::new("nonexistent_model")?;
    tracing::debug!(model = "nonexistent_model", "Created OllamaClient with nonexistent model");

    // Should fail - model doesn't exist
    tracing::debug!("Attempting to validate nonexistent model");
    let result = client.validate().await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e.kind(), OllamaErrorKind::ModelNotFound(_)));
        tracing::debug!(error_kind = ?e.kind(), "Validation correctly failed with ModelNotFound");
    }
    
    tracing::info!("Ollama model validation test passed");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_ollama_server_not_running() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Ollama server not running");
    
    // Use non-standard port where Ollama is unlikely to be running
    let client = OllamaClient::new_with_url("llama2", "http://localhost:11435")?;
    tracing::debug!(model = "llama2", url = "http://localhost:11435", "Created OllamaClient with non-standard port");

    tracing::debug!("Attempting to validate (should fail - server not running)");
    let result = client.validate().await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e.kind(), OllamaErrorKind::ServerNotRunning(_)));
        tracing::debug!(error_kind = ?e.kind(), "Validation correctly failed with ServerNotRunning");
    }
    
    tracing::info!("Ollama server not running test passed");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_ollama_multi_message_conversation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Ollama multi-message conversation");
    
    let client = OllamaClient::new("llama2")?;
    tracing::debug!(model = "llama2", "Created OllamaClient");

    tracing::debug!("Validating Ollama server and model");
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
    tracing::debug!(message_count = request.messages().len(), "Created multi-message request");

    tracing::debug!("Sending generation request");
    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response: {:?}", response.outputs());
    tracing::debug!(output_count = response.outputs().len(), "Received response");
    
    tracing::info!("Ollama multi-message conversation test passed");
    Ok(())
}
