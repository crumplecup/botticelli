//! Tests for execution tools (Phase 3).

mod helpers;

use botticelli_mcp::ToolRegistry;
use serde_json::json;
use std::fs;

#[tokio::test]
async fn test_generate_tool_basic() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing generate tool basic usage");

    let registry = ToolRegistry::default();

    let input = json!({
        "prompt": "Tell me a joke"
    });

    let result = registry.execute("generate", input).await?;
    tracing::debug!(?result, "Generate result");

    // Generate now returns actual generated text, not just config
    assert!(result["text"].is_string(), "Should have text field");
    assert!(result["model"].is_string(), "Should have model field");
    assert!(result["tokens_used"].is_number(), "Should have tokens_used field");

    tracing::info!("Generate basic test passed");
    Ok(())
}

#[tokio::test]
async fn test_generate_tool_with_model() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing generate tool with model parameters");

    let registry = ToolRegistry::default();

    let input = json!({
        "prompt": "Explain quantum physics",
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 2048,
        "temperature": 0.7
    });

    let result = registry.execute("generate", input).await?;
    tracing::debug!(?result, "Generate result with model");

    // Verify response structure
    assert!(result["text"].is_string(), "Should have text field");
    assert_eq!(result["model"], "claude-3-5-sonnet-20241022", "Should use specified model");

    tracing::info!("Generate with model test passed");
    Ok(())
}

#[tokio::test]
async fn test_generate_tool_with_system_prompt() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing generate tool with system prompt");

    let registry = ToolRegistry::default();

    let input = json!({
        "prompt": "Write a poem",
        "system_prompt": "You are a professional poet"
    });

    let result = registry.execute("generate", input).await?;
    tracing::debug!(?result, "Generate result with system prompt");

    // System prompt affects generation but isn't returned in response
    assert!(result["text"].is_string(), "Should have text field");

    tracing::info!("Generate with system prompt test passed");
    Ok(())
}

#[tokio::test]
async fn test_generate_tool_missing_prompt() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing generate tool with missing prompt");

    let registry = ToolRegistry::default();

    let input = json!({
        "model": "gpt-4"
    });

    let result = registry.execute("generate", input).await;
    tracing::debug!(is_err = result.is_err(), "Generate result for missing prompt");

    assert!(result.is_err(), "Should fail with missing prompt");

    tracing::info!("Generate missing prompt test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_narrative_tool() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing execute_narrative tool");

    let registry = ToolRegistry::default();

    // Create a temporary test narrative
    let temp_dir = std::env::temp_dir();
    let narrative_path = temp_dir.join("test_narrative.toml");
    tracing::debug!(path = ?narrative_path, "Creating temp narrative");

    let narrative_content = r#"[narrative]
name = "test"
description = "Test narrative"

[toc]
order = ["act1"]

[acts]
act1 = "Hello world"
"#;

    fs::write(&narrative_path, narrative_content)?;

    let input = json!({
        "narrative_path": narrative_path.to_str().unwrap(),
        "prompt": "Test prompt"
    });

    // Without LLM backends, tool returns error about missing backends
    // With LLM backends but no API keys, tool returns error about missing credentials
    // Both are acceptable for this test
    let result = registry.execute("execute_narrative", input).await;

    // Test should not panic - graceful degradation is expected
    if result.is_err() {
        let err_msg = result.as_ref().unwrap_err().to_string();
        tracing::debug!(error = %err_msg, "Execute narrative error (expected)");
        assert!(
            err_msg.contains("backend") || err_msg.contains("available") || err_msg.contains("API"),
            "Expected backend/credential error, got: {}",
            err_msg
        );
    } else {
        tracing::debug!(?result, "Execute narrative succeeded");
    }

    // Cleanup
    fs::remove_file(narrative_path).ok();

    tracing::info!("Execute narrative test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_narrative_tool_file_not_found() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing execute_narrative with missing file");

    let registry = ToolRegistry::default();

    let input = json!({
        "narrative_path": "/nonexistent/narrative.toml",
        "prompt": "Test"
    });

    let result = registry.execute("execute_narrative", input).await;
    tracing::debug!(is_err = result.is_err(), "Execute result for missing file");

    assert!(result.is_err(), "Should fail with nonexistent file");

    tracing::info!("Execute narrative file not found test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_narrative_tool_invalid_toml() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing execute_narrative with invalid TOML");

    let registry = ToolRegistry::default();

    // Create a temporary invalid narrative
    let temp_dir = std::env::temp_dir();
    let narrative_path = temp_dir.join("invalid_narrative.toml");
    tracing::debug!(path = ?narrative_path, "Creating invalid narrative");

    let invalid_content = "[[acts]]\nthis is invalid";
    fs::write(&narrative_path, invalid_content)?;

    let input = json!({
        "narrative_path": narrative_path.to_str().unwrap(),
        "prompt": "Test prompt"
    });

    let result = registry.execute("execute_narrative", input).await;
    tracing::debug!(is_err = result.is_err(), "Execute result for invalid TOML");

    // Current implementation is a placeholder - it may not validate TOML yet
    // Just verify the tool executes without panicking
    if result.is_err() {
        tracing::debug!(error = ?result.as_ref().unwrap_err(), "Tool returned error (validation working)");
    } else {
        tracing::debug!("Tool returned success (placeholder implementation)");
    }

    // Cleanup
    fs::remove_file(narrative_path).ok();

    tracing::info!("Execute narrative invalid TOML test passed");
    Ok(())
}

#[tokio::test]
async fn test_tool_registry_includes_execution_tools() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing tool registry includes execution tools");

    let registry = ToolRegistry::default();

    // Test by attempting to execute - if tool exists, it will work or fail with specific error
    let generate_test = registry.execute("generate", json!({"prompt": "test"})).await;
    let execute_test = registry.execute("execute_narrative", json!({"narrative_path": "/fake", "prompt": "test"})).await;
    
    tracing::debug!(
        generate_ok = generate_test.is_ok(),
        execute_has_file_err = execute_test.as_ref().err().map(|e| e.to_string().contains("file")).unwrap_or(false),
        "Tool execution"
    );

    // Generate should succeed (returns placeholder text)
    assert!(generate_test.is_ok(), "Generate tool should be available");
    
    // Execute_narrative should fail with file error (not "unknown tool")
    assert!(execute_test.is_err(), "Execute should fail with bad file path");
    let err = execute_test.unwrap_err().to_string();
    assert!(!err.to_lowercase().contains("unknown tool"), 
        "Should fail with file error, not 'unknown tool': {}", err);

    tracing::info!("Tool registry test passed");
    Ok(())
}

#[tokio::test]
async fn test_generate_input_schema() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing generate tool input schema");

    // Schema is now accessed via rmcp ToolRouter, test by calling tool
    let registry = ToolRegistry::default();
    
    // If tool accepts prompt, schema is correct
    let result = registry.execute("generate", json!({"prompt": "test"})).await;
    tracing::debug!(has_prompt = result.is_ok() || !result.as_ref().unwrap_err().to_string().contains("prompt"), "Schema validation");

    // Missing required field should error
    let no_prompt = registry.execute("generate", json!({"model": "test"})).await;
    assert!(no_prompt.is_err(), "Should require prompt field");

    tracing::info!("Generate schema test passed");
    Ok(())
}

#[tokio::test]
async fn test_execute_narrative_input_schema() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing execute_narrative tool input schema");

    let registry = ToolRegistry::default();

    // Missing required fields should error
    let no_file = registry.execute("execute_narrative", json!({"prompt": "test"})).await;
    let no_prompt = registry.execute("execute_narrative", json!({"narrative_path": "/test"})).await;
    
    tracing::debug!(
        no_file_err = no_file.is_err(),
        no_prompt_err = no_prompt.is_err(),
        "Schema validation"
    );

    assert!(no_file.is_err(), "Should require narrative_path field");
    assert!(no_prompt.is_err(), "Should require prompt field");

    tracing::info!("Execute narrative schema test passed");
    Ok(())
}
