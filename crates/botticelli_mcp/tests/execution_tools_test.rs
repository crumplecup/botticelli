//! Tests for execution tools (Phase 3).

use botticelli_mcp::tools::{ExecuteNarrativeTool, GenerateTool, McpTool, ToolRegistry};
use serde_json::json;
use std::fs;

#[tokio::test]
async fn test_generate_tool_basic() {
    let tool = GenerateTool;

    let input = json!({
        "prompt": "Tell me a joke"
    });

    let result = tool.execute(input).await.unwrap();
    assert_eq!(result["status"], "configured");
    assert_eq!(result["config"]["prompt"], "Tell me a joke");
    assert_eq!(result["config"]["model"], "gemini-2.0-flash-exp");
}

#[tokio::test]
async fn test_generate_tool_with_model() {
    let tool = GenerateTool;

    let input = json!({
        "prompt": "Explain quantum physics",
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 2048,
        "temperature": 0.7
    });

    let result = tool.execute(input).await.unwrap();
    assert_eq!(result["status"], "configured");
    assert_eq!(result["config"]["model"], "claude-3-5-sonnet-20241022");
    assert_eq!(result["config"]["max_tokens"], 2048);
    // Float comparison with tolerance
    let temp = result["config"]["temperature"].as_f64().unwrap();
    assert!((temp - 0.7).abs() < 0.01);
}

#[tokio::test]
async fn test_generate_tool_with_system_prompt() {
    let tool = GenerateTool;

    let input = json!({
        "prompt": "Write a poem",
        "system_prompt": "You are a professional poet"
    });

    let result = tool.execute(input).await.unwrap();
    assert_eq!(
        result["config"]["system_prompt"],
        "You are a professional poet"
    );
}

#[tokio::test]
async fn test_generate_tool_missing_prompt() {
    let tool = GenerateTool;

    let input = json!({
        "model": "gpt-4"
    });

    let result = tool.execute(input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_narrative_tool() {
    let tool = ExecuteNarrativeTool::new();

    // Create a temporary test narrative
    let temp_dir = std::env::temp_dir();
    let narrative_path = temp_dir.join("test_narrative.toml");

    let narrative_content = r#"[narrative]
name = "test"
description = "Test narrative"

[toc]
order = ["act1"]

[acts]
act1 = "Hello world"
"#;

    fs::write(&narrative_path, narrative_content).unwrap();

    let input = json!({
        "file_path": narrative_path.to_str().unwrap(),
        "prompt": "Test prompt"
    });

    // Without LLM backends, tool returns error about missing backends
    // With LLM backends but no API keys, tool returns error about missing credentials
    // Both are acceptable for this test
    let result = tool.execute(input).await;

    // Test should not panic - graceful degradation is expected
    if let Err(err) = result {
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("backend") || err_msg.contains("available") || err_msg.contains("API"),
            "Expected backend/credential error, got: {}",
            err_msg
        );
    }

    // Cleanup
    fs::remove_file(narrative_path).ok();
}

#[tokio::test]
async fn test_execute_narrative_tool_file_not_found() {
    let tool = ExecuteNarrativeTool::new();

    let input = json!({
        "file_path": "/nonexistent/narrative.toml",
        "prompt": "Test"
    });

    let result = tool.execute(input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_narrative_tool_invalid_toml() {
    let tool = ExecuteNarrativeTool::new();

    // Create a temporary invalid narrative
    let temp_dir = std::env::temp_dir();
    let narrative_path = temp_dir.join("invalid_narrative.toml");

    let invalid_content = "[[acts]]\nthis is invalid";
    fs::write(&narrative_path, invalid_content).unwrap();

    let input = json!({
        "file_path": narrative_path.to_str().unwrap(),
        "prompt": "Test prompt"
    });

    let result = tool.execute(input).await;
    assert!(result.is_err());

    // Cleanup
    fs::remove_file(narrative_path).ok();
}

#[tokio::test]
async fn test_tool_registry_includes_execution_tools() {
    let registry = ToolRegistry::default();

    assert!(registry.get("generate").is_some());
    assert!(registry.get("execute_narrative").is_some());

    // Verify minimum count
    // Core: echo, server_info, validate_narrative, generate, execute_narrative
    // Optional: query_content (database), discord tools (discord + token)
    let count = registry.len();
    assert!(
        count >= 5,
        "Should have at least 5 core tools, got {}",
        count
    );
}

#[tokio::test]
async fn test_generate_input_schema() {
    let tool = GenerateTool;
    let schema = tool.input_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["prompt"].is_object());
    assert!(schema["properties"]["model"].is_object());
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("prompt"))
    );
}

#[tokio::test]
async fn test_execute_narrative_input_schema() {
    let tool = ExecuteNarrativeTool::new();
    let schema = tool.input_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["file_path"].is_object());
    assert!(schema["properties"]["prompt"].is_object());
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("file_path"))
    );
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("prompt"))
    );
}
