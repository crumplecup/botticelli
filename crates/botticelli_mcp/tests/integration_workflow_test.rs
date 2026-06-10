//! End-to-end integration tests for MCP workflow.

use botticelli_mcp::tools::{
    ExecuteNarrativeTool, GenerateTool, McpTool, ToolRegistry, ValidateNarrativeTool,
};
use serde_json::json;
use std::fs;

/// Test the complete workflow: validate → generate → execute
#[tokio::test]
async fn test_complete_narrative_workflow() {
    // Step 1: Create a test narrative
    let narrative_content = r#"
[narrative]
name = "workflow_test"
description = "Test workflow"
model = "gemini-2.0-flash-exp"

[toc]
order = ["greeting", "question"]

[acts]
greeting = "Say hello"
question = "Ask about the weather"
"#;

    // Step 2: Validate the narrative
    let validator = ValidateNarrativeTool;
    let validation_input = json!({
        "content": narrative_content,
        "validate_models": true,
        "warn_unused": true
    });

    let validation_result = validator.execute(validation_input).await.unwrap();
    assert_eq!(validation_result["valid"], true);
    assert_eq!(validation_result["errors"].as_array().unwrap().len(), 0);

    // Step 3: Write narrative to temp file
    let temp_dir = std::env::temp_dir();
    let narrative_path = temp_dir.join("workflow_test.toml");
    fs::write(&narrative_path, narrative_content).unwrap();

    // Step 4: Execute the narrative (will fail without API keys, but validates flow)
    let executor = ExecuteNarrativeTool::new();
    let execution_input = json!({
        "file_path": narrative_path.to_str().unwrap(),
        "prompt": "Test workflow execution"
    });

    let execution_result = executor.execute(execution_input).await;

    // Should fail gracefully without API keys
    if let Err(err) = execution_result {
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("backend") || err_msg.contains("available") || err_msg.contains("API")
        );
    }

    // Cleanup
    fs::remove_file(narrative_path).ok();
}

/// Test validation catches common syntax errors
#[tokio::test]
async fn test_validation_error_handling() {
    let validator = ValidateNarrativeTool;

    // Test various invalid narratives
    let test_cases = vec![
        (
            "array_acts",
            r#"
[narrative]
name = "test"

[toc]
order = ["act1"]

[[acts]]
name = "act1"
prompt = "Hello"
"#,
            "[[acts]]",
        ),
        (
            "missing_act",
            r#"
[narrative]
name = "test"

[toc]
order = ["act1", "act2"]

[acts]
act1 = "Hello"
"#,
            "act2",
        ),
        (
            "missing_toc",
            r#"
[narrative]
name = "test"

[acts]
act1 = "Hello"
"#,
            "toc",
        ),
    ];

    for (name, content, expected_error) in test_cases {
        let input = json!({
            "content": content
        });

        let result = validator.execute(input).await.unwrap();
        assert_eq!(
            result["valid"], false,
            "Test case '{}' should be invalid",
            name
        );

        let errors = result["errors"].as_array().unwrap();
        assert!(
            !errors.is_empty(),
            "Test case '{}' should have errors",
            name
        );

        let error_msg = errors[0]["message"].as_str().unwrap().to_lowercase();
        assert!(
            error_msg.contains(&expected_error.to_lowercase()),
            "Test case '{}' error should mention '{}', got: {}",
            name,
            expected_error,
            error_msg
        );
    }
}

/// Test tool registry provides all expected tools
#[tokio::test]
async fn test_tool_registry_completeness() {
    let registry = ToolRegistry::default();

    // Core tools (always available)
    assert!(registry.get("echo").is_some());
    assert!(registry.get("validate_narrative").is_some());
    assert!(registry.get("generate").is_some());
    assert!(registry.get("execute_act").is_some());
    assert!(registry.get("execute_narrative").is_some());

    // Database tool (feature-gated)
    #[cfg(feature = "database")]
    assert!(registry.get("query_content").is_some());

    // Discord tools (feature-gated)
    #[cfg(feature = "discord")]
    {
        if std::env::var("DISCORD_TOKEN").is_ok() {
            assert!(registry.get("discord_post_message").is_some());
            assert!(registry.get("discord_get_messages").is_some());
            assert!(registry.get("discord_get_guild_info").is_some());
            assert!(registry.get("discord_get_channels").is_some());
        }
    }
}

/// Test generate tool configuration
#[tokio::test]
async fn test_generate_tool_configuration() {
    let tool = GenerateTool;

    // Test various model configurations
    let models = vec![
        "gemini-2.0-flash-exp",
        "claude-3-5-sonnet-20241022",
        "gpt-4-turbo",
        "llama-3.3-70b-versatile",
    ];

    for model in models {
        let input = json!({
            "prompt": "Test prompt",
            "model": model,
            "max_tokens": 100,
            "temperature": 0.5
        });

        let result = tool.execute(input).await.unwrap();
        assert_eq!(result["status"], "configured");
        assert_eq!(result["config"]["model"], model);
        assert_eq!(result["config"]["max_tokens"], 100);

        let temp = result["config"]["temperature"].as_f64().unwrap();
        assert!((temp - 0.5).abs() < 0.01);
    }
}

/// Test validation with model checking
#[tokio::test]
async fn test_validation_model_checking() {
    let validator = ValidateNarrativeTool;

    // Test with known model
    let valid_input = json!({
        "content": r#"
[narrative]
name = "test"
model = "gemini-2.0-flash-exp"

[toc]
order = ["act1"]

[acts]
act1 = "Hello"
"#,
        "validate_models": true
    });

    let result = validator.execute(valid_input).await.unwrap();
    assert_eq!(result["valid"], true);
    assert_eq!(result["warnings"].as_array().unwrap().len(), 0);

    // Test with unknown model
    let unknown_input = json!({
        "content": r#"
[narrative]
name = "test"
model = "unknown-model-xyz"

[toc]
order = ["act1"]

[acts]
act1 = "Hello"
"#,
        "validate_models": true
    });

    let result = validator.execute(unknown_input).await.unwrap();
    assert_eq!(result["valid"], true); // Warnings don't fail validation
    assert!(!result["warnings"].as_array().unwrap().is_empty());
}

/// Test validation in strict mode
#[tokio::test]
async fn test_validation_strict_mode() {
    let validator = ValidateNarrativeTool;

    let input = json!({
        "content": r#"
[narrative]
name = "test"
model = "unknown-model"

[bots.unused_bot]
platform = "discord"
command = "test"

[toc]
order = ["act1"]

[acts]
act1 = "Hello"
"#,
        "strict": true,
        "validate_models": true,
        "warn_unused": true
    });

    let result = validator.execute(input).await.unwrap();

    // Strict mode treats warnings as errors
    assert_eq!(result["valid"], false);

    // Should have warnings (treated as errors in strict mode)
    assert!(!result["warnings"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_execution_with_invalid_narrative() {
    let executor = ExecuteNarrativeTool::new();

    // Create invalid narrative
    let temp_dir = std::env::temp_dir();
    let narrative_path = temp_dir.join("invalid_exec.toml");

    let invalid_content = r#"
[narrative]
name = "test"

[toc]
order = ["missing_act"]

[acts]
different_act = "Hello"
"#;

    fs::write(&narrative_path, invalid_content).unwrap();

    let input = json!({
        "file_path": narrative_path.to_str().unwrap(),
        "prompt": "Test"
    });

    let result = executor.execute(input).await;
    assert!(result.is_err(), "Should fail with invalid narrative");

    // Cleanup
    fs::remove_file(narrative_path).ok();
}

/// Test tool schemas are well-formed
#[tokio::test]
async fn test_tool_schemas() {
    let registry = ToolRegistry::default();

    for tool in registry.list() {
        let name = tool.name();
        let schema = tool.input_schema();

        // All schemas must be objects
        assert_eq!(
            schema.get("type").and_then(|v| v.as_str()),
            Some("object"),
            "Tool '{}' schema must be object type",
            name
        );

        // Must have properties
        assert!(
            schema.get("properties").is_some(),
            "Tool '{}' must have properties",
            name
        );

        // If required exists, must be array
        if let Some(required) = schema.get("required") {
            assert!(
                required.is_array(),
                "Tool '{}' required must be array",
                name
            );
        }
    }
}
