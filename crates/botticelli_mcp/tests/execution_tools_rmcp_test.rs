//! Tests for execution tools (generate, execute_act, execute_narrative).

mod helpers;

use botticelli_mcp::{BotticelliServer, ExecuteActParams, ExecuteNarrativeParams, GenerateParams};
use rmcp::handler::server::wrapper::Parameters;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_generate_basic() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing basic generate");

    let server = BotticelliServer::builder().build()?;

    let params = GenerateParams::new(
        "Hello, world!".to_string(),
        "gemini-2.0-flash-exp".to_string(),
        100,
        0.7,
        None,
    );

    let result = server.generate(Parameters(params)).await?;
    tracing::debug!(text_len = result.0.text().len(), model = %result.0.model(), "Generate result");

    assert!(!result.0.text().is_empty(), "Should return generated text");
    assert_eq!(result.0.model(), "gemini-2.0-flash-exp");

    tracing::info!("Basic generate test passed");
    Ok(())
}

#[tokio::test]
async fn test_generate_with_system_prompt() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing generate with system prompt");

    let server = BotticelliServer::builder().build()?;

    let params = GenerateParams::new(
        "Write a haiku".to_string(),
        "claude-3-5-sonnet-20241022".to_string(),
        50,
        0.9,
        Some("You are a poetry expert.".to_string()),
    );

    let result = server.generate(Parameters(params)).await?;
    tracing::debug!(
        has_system_prompt = true,
        text_len = result.0.text().len(),
        "Generate result"
    );

    assert!(!result.0.text().is_empty());
    assert_eq!(result.0.model(), "claude-3-5-sonnet-20241022");

    tracing::info!("Generate with system prompt test passed");
    Ok(())
}

#[tokio::test]
async fn test_generate_default_values() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing generate with default values");

    let server = BotticelliServer::builder().build()?;

    let params = GenerateParams::new(
        "Test prompt".to_string(),
        "gemini-2.0-flash-exp".to_string(), // default
        1024,                               // default
        1.0,                                // default
        None,
    );

    let result = server.generate(Parameters(params)).await?;
    tracing::debug!(model = %result.0.model(), "Generate result");

    assert!(!result.0.text().is_empty());

    tracing::info!("Generate default values test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_act_basic() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing basic execute_act");

    let server = BotticelliServer::builder().build()?;

    let params = ExecuteActParams::new(
        "Analyze this data".to_string(),
        "gemini-2.0-flash-exp".to_string(),
        200,
        0.5,
        None,
        None,
    );

    let result = server.execute_act(Parameters(params)).await?;
    tracing::debug!(
        response_len = result.0.response().len(),
        success = result.0.success(),
        model = %result.0.model(),
        "Execute act result"
    );

    assert!(!result.0.response().is_empty(), "Should return response");
    assert!(result.0.success(), "Should be successful");
    assert_eq!(result.0.model(), "gemini-2.0-flash-exp");

    tracing::info!("Basic execute_act test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_act_with_context() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing execute_act with context");

    let server = BotticelliServer::builder().build()?;

    let params = ExecuteActParams::new(
        "Continue the story".to_string(),
        "gpt-4o".to_string(),
        150,
        0.8,
        Some("You are a storyteller.".to_string()),
        Some("Once upon a time...".to_string()),
    );

    let result = server.execute_act(Parameters(params)).await?;
    tracing::debug!(
        response_len = result.0.response().len(),
        success = result.0.success(),
        "Execute act result"
    );

    assert!(!result.0.response().is_empty());
    assert!(result.0.success());

    tracing::info!("Execute_act with context test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_narrative_file_not_found() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing execute_narrative with missing file");

    let server = BotticelliServer::builder().build()?;

    let params = ExecuteNarrativeParams::new(
        "/nonexistent/narrative.toml".to_string(),
        "Test prompt".to_string(),
        None,
        1024,
    );

    let result = server.execute_narrative(Parameters(params)).await;
    tracing::debug!(is_err = result.is_err(), "Execute narrative result");

    assert!(result.is_err(), "Should fail with file not found");

    tracing::info!("Execute_narrative file not found test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_narrative_with_file() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing execute_narrative with file");

    let server = BotticelliServer::builder().build()?;
    let temp_dir = TempDir::new()?;
    let narrative_path = temp_dir.path().join("test_narrative.toml");
    tracing::debug!(path = ?narrative_path, "Created temp narrative path");

    // Create a minimal narrative file
    let narrative_toml = r#"
title = "Test Narrative"
version = "0.1.0"

[[acts]]
name = "intro"
objective = "Introduce the story"
"#;

    fs::write(&narrative_path, narrative_toml).await?;

    let params = ExecuteNarrativeParams::new(
        narrative_path.to_string_lossy().to_string(),
        "Start the narrative".to_string(),
        Some("gemini-2.0-flash-exp".to_string()),
        500,
    );

    let result = server.execute_narrative(Parameters(params)).await?;
    tracing::debug!(
        output_len = result.0.final_output().len(),
        acts_executed = result.0.acts_executed(),
        success = result.0.success(),
        "Execute narrative result"
    );

    assert!(!result.0.final_output().is_empty(), "Should return output");
    assert!(result.0.success(), "Should be successful");
    assert!(!result.0.models_used().is_empty(), "Should have models");

    tracing::info!("Execute_narrative with file test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_narrative_default_model() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing execute_narrative with default model");

    let server = BotticelliServer::builder().build()?;
    let temp_dir = TempDir::new()?;
    let narrative_path = temp_dir.path().join("test.toml");

    fs::write(&narrative_path, "title = \"Test\"\nversion = \"0.1.0\"\n").await?;

    let params = ExecuteNarrativeParams::new(
        narrative_path.to_string_lossy().to_string(),
        "Test".to_string(),
        None, // Should use default
        1024,
    );

    let result = server.execute_narrative(Parameters(params)).await?;
    tracing::debug!(models_used = ?result.0.models_used(), "Execute narrative result");

    assert!(!result.0.models_used().is_empty());
    // Default model should be used
    assert_eq!(result.0.models_used()[0], "gemini-2.0-flash-exp");

    tracing::info!("Execute_narrative default model test passed");
    Ok(())
}

#[tokio::test]
async fn test_generate_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing GenerateParams serialization");

    let params = GenerateParams::new(
        "Test".to_string(),
        "gpt-4o".to_string(),
        100,
        0.5,
        Some("System".to_string()),
    );

    let json = serde_json::to_value(&params)?;
    tracing::debug!(?json, "Serialized params");

    assert!(json.is_object());
    assert_eq!(json["prompt"], "Test");
    assert_eq!(json["model"], "gpt-4o");
    assert_eq!(json["max_tokens"], 100);

    tracing::info!("GenerateParams serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_generate_result_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing GenerateResult serialization");

    use botticelli_mcp::GenerateResult;

    let result = GenerateResult::new(
        "Generated text".to_string(),
        "gemini-2.0-flash-exp".to_string(),
        Some(50),
    );

    let json = serde_json::to_value(&result)?;
    tracing::debug!(?json, "Serialized result");

    assert!(json.is_object());
    assert_eq!(json["text"], "Generated text");
    assert_eq!(json["model"], "gemini-2.0-flash-exp");
    assert_eq!(json["tokens_used"], 50);

    tracing::info!("GenerateResult serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_act_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ExecuteActParams serialization");

    let params = ExecuteActParams::new(
        "Act prompt".to_string(),
        "claude-3-5-sonnet-20241022".to_string(),
        200,
        0.7,
        Some("System".to_string()),
        Some("Context".to_string()),
    );

    let json = serde_json::to_value(&params)?;
    tracing::debug!(?json, "Serialized params");

    assert!(json.is_object());
    assert_eq!(json["prompt"], "Act prompt");
    assert_eq!(json["context"], "Context");

    tracing::info!("ExecuteActParams serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_act_result_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ExecuteActResult serialization");

    use botticelli_mcp::ExecuteActResult;

    let result = ExecuteActResult::new(
        "Response".to_string(),
        "gpt-4o".to_string(),
        Some(100),
        true,
    );

    let json = serde_json::to_value(&result)?;
    tracing::debug!(?json, "Serialized result");

    assert!(json.is_object());
    assert_eq!(json["response"], "Response");
    assert_eq!(json["success"], true);

    tracing::info!("ExecuteActResult serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_narrative_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ExecuteNarrativeParams serialization");

    let params = ExecuteNarrativeParams::new(
        "/path/to/narrative.toml".to_string(),
        "Prompt".to_string(),
        Some("gemini".to_string()),
        1024,
    );

    let json = serde_json::to_value(&params)?;
    tracing::debug!(?json, "Serialized params");

    assert!(json.is_object());
    assert_eq!(json["narrative_path"], "/path/to/narrative.toml");
    assert_eq!(json["model"], "gemini");

    tracing::info!("ExecuteNarrativeParams serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_narrative_result_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ExecuteNarrativeResult serialization");

    use botticelli_mcp::ExecuteNarrativeResult;

    let result = ExecuteNarrativeResult::new(
        "Final output".to_string(),
        3,
        vec!["model1".to_string(), "model2".to_string()],
        Some(500),
        true,
        None,
    );

    let json = serde_json::to_value(&result)?;
    tracing::debug!(?json, "Serialized result");

    assert!(json.is_object());
    assert_eq!(json["final_output"], "Final output");
    assert_eq!(json["acts_executed"], 3);
    assert!(json["models_used"].is_array());
    assert_eq!(json["success"], true);

    tracing::info!("ExecuteNarrativeResult serialization test passed");
    Ok(())
}
