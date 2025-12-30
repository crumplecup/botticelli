//! Tests for execution tools (generate, execute_act, execute_narrative).

use botticelli_mcp::{
    BotticelliServer, ExecuteActParams, ExecuteNarrativeParams, GenerateParams,
};
use rmcp::handler::server::wrapper::Parameters;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_generate_basic() {
    let server = BotticelliServer::builder().build();

    let params = GenerateParams {
        prompt: "Hello, world!".to_string(),
        model: "gemini-2.0-flash-exp".to_string(),
        max_tokens: 100,
        temperature: 0.7,
        system_prompt: None,
    };

    let result = server
        .generate(Parameters(params))
        .await
        .expect("Generate should succeed");

    assert!(!result.0.text.is_empty(), "Should return generated text");
    assert_eq!(result.0.model, "gemini-2.0-flash-exp");
}

#[tokio::test]
async fn test_generate_with_system_prompt() {
    let server = BotticelliServer::builder().build();

    let params = GenerateParams {
        prompt: "Write a haiku".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 50,
        temperature: 0.9,
        system_prompt: Some("You are a poetry expert.".to_string()),
    };

    let result = server
        .generate(Parameters(params))
        .await
        .expect("Generate with system prompt should succeed");

    assert!(!result.0.text.is_empty());
    assert_eq!(result.0.model, "claude-3-5-sonnet-20241022");
}

#[tokio::test]
async fn test_generate_default_values() {
    let server = BotticelliServer::builder().build();

    let params = GenerateParams {
        prompt: "Test prompt".to_string(),
        model: "gemini-2.0-flash-exp".to_string(), // default
        max_tokens: 1024,                            // default
        temperature: 1.0,                            // default
        system_prompt: None,
    };

    let result = server
        .generate(Parameters(params))
        .await
        .expect("Generate with defaults should succeed");

    assert!(!result.0.text.is_empty());
}

#[tokio::test]
async fn test_execute_act_basic() {
    let server = BotticelliServer::builder().build();

    let params = ExecuteActParams {
        prompt: "Analyze this data".to_string(),
        model: "gemini-2.0-flash-exp".to_string(),
        max_tokens: 200,
        temperature: 0.5,
        system_prompt: None,
        context: None,
    };

    let result = server
        .execute_act(Parameters(params))
        .await
        .expect("Execute act should succeed");

    assert!(!result.0.response.is_empty(), "Should return response");
    assert!(result.0.success, "Should be successful");
    assert_eq!(result.0.model, "gemini-2.0-flash-exp");
}

#[tokio::test]
async fn test_execute_act_with_context() {
    let server = BotticelliServer::builder().build();

    let params = ExecuteActParams {
        prompt: "Continue the story".to_string(),
        model: "gpt-4o".to_string(),
        max_tokens: 150,
        temperature: 0.8,
        system_prompt: Some("You are a storyteller.".to_string()),
        context: Some("Once upon a time...".to_string()),
    };

    let result = server
        .execute_act(Parameters(params))
        .await
        .expect("Execute act with context should succeed");

    assert!(!result.0.response.is_empty());
    assert!(result.0.success);
}

#[tokio::test]
async fn test_execute_narrative_file_not_found() {
    let server = BotticelliServer::builder().build();

    let params = ExecuteNarrativeParams {
        narrative_path: "/nonexistent/narrative.toml".to_string(),
        prompt: "Test prompt".to_string(),
        model: None,
        max_tokens: 1024,
    };

    let result = server.execute_narrative(Parameters(params)).await;

    assert!(result.is_err(), "Should fail with file not found");
}

#[tokio::test]
async fn test_execute_narrative_with_file() {
    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new().unwrap();
    let narrative_path = temp_dir.path().join("test_narrative.toml");

    // Create a minimal narrative file
    let narrative_toml = r#"
title = "Test Narrative"
version = "0.1.0"

[[acts]]
name = "intro"
objective = "Introduce the story"
"#;

    fs::write(&narrative_path, narrative_toml).await.unwrap();

    let params = ExecuteNarrativeParams {
        narrative_path: narrative_path.to_string_lossy().to_string(),
        prompt: "Start the narrative".to_string(),
        model: Some("gemini-2.0-flash-exp".to_string()),
        max_tokens: 500,
    };

    let result = server
        .execute_narrative(Parameters(params))
        .await
        .expect("Execute narrative should succeed");

    assert!(!result.0.final_output.is_empty(), "Should return output");
    assert!(result.0.success, "Should be successful");
    assert!(!result.0.models_used.is_empty(), "Should have models");
}

#[tokio::test]
async fn test_execute_narrative_default_model() {
    let server = BotticelliServer::builder().build();
    let temp_dir = TempDir::new().unwrap();
    let narrative_path = temp_dir.path().join("test.toml");

    fs::write(&narrative_path, "title = \"Test\"\nversion = \"0.1.0\"\n")
        .await
        .unwrap();

    let params = ExecuteNarrativeParams {
        narrative_path: narrative_path.to_string_lossy().to_string(),
        prompt: "Test".to_string(),
        model: None, // Should use default
        max_tokens: 1024,
    };

    let result = server
        .execute_narrative(Parameters(params))
        .await
        .expect("Should succeed");

    assert!(!result.0.models_used.is_empty());
    // Default model should be used
    assert_eq!(result.0.models_used[0], "gemini-2.0-flash-exp");
}

#[tokio::test]
async fn test_generate_params_serialization() {
    let params = GenerateParams {
        prompt: "Test".to_string(),
        model: "gpt-4o".to_string(),
        max_tokens: 100,
        temperature: 0.5,
        system_prompt: Some("System".to_string()),
    };

    let json = serde_json::to_value(&params).unwrap();
    assert!(json.is_object());
    assert_eq!(json["prompt"], "Test");
    assert_eq!(json["model"], "gpt-4o");
    assert_eq!(json["max_tokens"], 100);
}

#[tokio::test]
async fn test_generate_result_serialization() {
    use botticelli_mcp::GenerateResult;

    let result = GenerateResult::new(
        "Generated text".to_string(),
        "gemini-2.0-flash-exp".to_string(),
        Some(50),
    );

    let json = serde_json::to_value(&result).unwrap();
    assert!(json.is_object());
    assert_eq!(json["text"], "Generated text");
    assert_eq!(json["model"], "gemini-2.0-flash-exp");
    assert_eq!(json["tokens_used"], 50);
}

#[tokio::test]
async fn test_execute_act_params_serialization() {
    let params = ExecuteActParams {
        prompt: "Act prompt".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 200,
        temperature: 0.7,
        system_prompt: Some("System".to_string()),
        context: Some("Context".to_string()),
    };

    let json = serde_json::to_value(&params).unwrap();
    assert!(json.is_object());
    assert_eq!(json["prompt"], "Act prompt");
    assert_eq!(json["context"], "Context");
}

#[tokio::test]
async fn test_execute_act_result_serialization() {
    use botticelli_mcp::ExecuteActResult;

    let result = ExecuteActResult::new(
        "Response".to_string(),
        "gpt-4o".to_string(),
        Some(100),
        true,
    );

    let json = serde_json::to_value(&result).unwrap();
    assert!(json.is_object());
    assert_eq!(json["response"], "Response");
    assert_eq!(json["success"], true);
}

#[tokio::test]
async fn test_execute_narrative_params_serialization() {
    let params = ExecuteNarrativeParams {
        narrative_path: "/path/to/narrative.toml".to_string(),
        prompt: "Prompt".to_string(),
        model: Some("gemini".to_string()),
        max_tokens: 1024,
    };

    let json = serde_json::to_value(&params).unwrap();
    assert!(json.is_object());
    assert_eq!(json["narrative_path"], "/path/to/narrative.toml");
    assert_eq!(json["model"], "gemini");
}

#[tokio::test]
async fn test_execute_narrative_result_serialization() {
    use botticelli_mcp::ExecuteNarrativeResult;

    let result = ExecuteNarrativeResult::new(
        "Final output".to_string(),
        3,
        vec!["model1".to_string(), "model2".to_string()],
        Some(500),
        true,
        None,
    );

    let json = serde_json::to_value(&result).unwrap();
    assert!(json.is_object());
    assert_eq!(json["final_output"], "Final output");
    assert_eq!(json["acts_executed"], 3);
    assert!(json["models_used"].is_array());
    assert_eq!(json["success"], true);
}
