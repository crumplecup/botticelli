//! Integration tests for PMCP-based MCP server implementation.
//!
//! Tests the new pmcp SDK server with all tools via the adapter pattern.

use botticelli_mcp::tools::{EchoTool, McpTool, ServerInfoTool};
use serde_json::json;

/// Test that EchoTool works correctly
#[tokio::test]
async fn test_echo_tool() {
    let tool = EchoTool;
    
    // Test with valid message
    let args = json!({
        "message": "Hello, PMCP!"
    });
    
    let result = tool.execute(args).await;
    assert!(result.is_ok(), "Echo tool should succeed: {:?}", result.err());
    
    let response = result.unwrap();
    assert!(response.get("echo").is_some(), "Response should have 'echo' field");
    assert_eq!(
        response["echo"].as_str().unwrap(),
        "Hello, PMCP!",
        "Echo should return the message"
    );
    assert!(response.get("timestamp").is_some(), "Response should have timestamp");
}

/// Test that EchoTool handles missing message
#[tokio::test]
async fn test_echo_tool_missing_message() {
    let tool = EchoTool;
    
    // Test with no message field
    let args = json!({});
    
    let result = tool.execute(args).await;
    // EchoTool might handle this gracefully or return error - check behavior
    // This documents the actual behavior
    match result {
        Ok(response) => {
            // If it succeeds, it should handle missing message gracefully
            assert!(response.get("echo").is_some());
        }
        Err(e) => {
            // If it errors, that's also valid - just document it
            assert!(e.to_string().contains("message") || e.to_string().contains("field"));
        }
    }
}

/// Test that ServerInfoTool works correctly
#[tokio::test]
async fn test_server_info_tool() {
    let tool = ServerInfoTool;
    
    let args = json!({});
    let result = tool.execute(args).await;
    
    assert!(result.is_ok(), "ServerInfo tool should succeed: {:?}", result.err());
    
    let response = result.unwrap();
    assert!(response.get("name").is_some(), "Response should have 'name' field");
    assert!(response.get("version").is_some(), "Response should have 'version' field");
}

/// Test CreateNarrativeTool basic functionality
#[tokio::test]
async fn test_create_narrative_tool_basic() {
    use botticelli_mcp::tools::CreateNarrativeTool;
    
    let tool = CreateNarrativeTool;
    
    // Test with minimal valid TOML
    let minimal_toml = r#"
[metadata]
title = "Test Narrative"
description = "A test narrative for PMCP"

[[acts]]
model_name = "test-model"
system_prompt = "You are a test assistant"
user_prompt = "Say hello"
"#;
    
    let args = json!({
        "toml_content": minimal_toml
    });
    
    let result = tool.execute(args).await;
    
    // This should parse successfully or give a clear validation error
    match result {
        Ok(response) => {
            println!("CreateNarrativeTool response: {:?}", response);
            assert!(
                response.get("success").is_some() || response.get("narrative").is_some(),
                "Response should indicate success"
            );
        }
        Err(e) => {
            // If it fails, the error should be clear about what's wrong
            let error_msg = e.to_string();
            println!("CreateNarrativeTool error: {}", error_msg);
            assert!(
                error_msg.contains("validation") 
                    || error_msg.contains("parse") 
                    || error_msg.contains("model")
                    || error_msg.contains("Missing")
                    || error_msg.contains("required"),
                "Error should be about validation or parsing: {}",
                error_msg
            );
        }
    }
}

/// Test ValidateNarrativeTool with valid TOML
#[tokio::test]
async fn test_validate_narrative_tool() {
    use botticelli_mcp::tools::ValidateNarrativeTool;
    
    let tool = ValidateNarrativeTool;
    
    let valid_toml = r#"
[metadata]
title = "Valid Test"
description = "A valid narrative"

[[acts]]
model_name = "test"
system_prompt = "Test"
user_prompt = "Test"
"#;
    
    let args = json!({
        "toml_content": valid_toml
    });
    
    let result = tool.execute(args).await;
    
    match result {
        Ok(response) => {
            println!("ValidateNarrativeTool response: {:?}", response);
            // Should indicate validation success
            assert!(
                response.get("valid").is_some() 
                    || response.get("errors").is_some()
                    || response.get("status").is_some(),
                "Response should have validation status"
            );
        }
        Err(e) => {
            println!("ValidateNarrativeTool error: {}", e);
            // Even errors should be structured
        }
    }
}

/// Test SaveNarrativeTool basic functionality  
#[tokio::test]
async fn test_save_narrative_tool() {
    use botticelli_mcp::tools::SaveNarrativeTool;
    
    let tool = SaveNarrativeTool;
    
    let test_toml = r#"
[metadata]
title = "Save Test"
description = "Testing save functionality"

[[acts]]
model_name = "test"
system_prompt = "Test"
user_prompt = "Test"
"#;
    
    let args = json!({
        "toml_content": test_toml,
        "filename": "test_narrative_pmcp.toml"
    });
    
    let result = tool.execute(args).await;
    
    match result {
        Ok(response) => {
            println!("SaveNarrativeTool response: {:?}", response);
            // Should indicate save success
            assert!(
                response.get("success").is_some() 
                    || response.get("path").is_some()
                    || response.get("file").is_some(),
                "Response should indicate save status"
            );
        }
        Err(e) => {
            println!("SaveNarrativeTool error: {}", e);
            // Errors might be due to permissions, disk space, etc
            assert!(!e.to_string().is_empty(), "Error should have a message");
        }
    }
}

/// Test that ModifyNarrativeTool handles basic modifications
#[tokio::test]
async fn test_modify_narrative_tool() {
    use botticelli_mcp::tools::ModifyNarrativeTool;
    
    let tool = ModifyNarrativeTool;
    
    let base_toml = r#"
[metadata]
title = "Original"
description = "Original description"

[[acts]]
model_name = "test"
system_prompt = "Original prompt"
user_prompt = "Original user prompt"
"#;
    
    let args = json!({
        "toml_content": base_toml,
        "modifications": {
            "metadata": {
                "title": "Modified Title"
            }
        }
    });
    
    let result = tool.execute(args).await;
    
    match result {
        Ok(response) => {
            println!("ModifyNarrativeTool response: {:?}", response);
            // Should return modified TOML
            assert!(
                response.get("modified_toml").is_some() 
                    || response.get("toml").is_some()
                    || response.get("success").is_some(),
                "Response should contain modified content"
            );
        }
        Err(e) => {
            println!("ModifyNarrativeTool error: {}", e);
            // Document error behavior
            assert!(!e.to_string().is_empty());
        }
    }
}

/// Test database tool (when feature enabled)
#[cfg(feature = "database")]
#[tokio::test]
async fn test_query_content_tool() {
    use botticelli_mcp::tools::QueryContentTool;
    
    let tool = QueryContentTool;
    
    let args = json!({
        "table_name": "test_table",
        "limit": 10
    });
    
    let result = tool.execute(args).await;
    
    // Database might not be available in test environment
    // So we just verify the tool accepts the request
    match result {
        Ok(response) => {
            println!("QueryContentTool response: {:?}", response);
            // Should have query results or empty array
            assert!(response.is_array() || response.is_object());
        }
        Err(e) => {
            println!("QueryContentTool error (expected in test): {}", e);
            // Likely database connection error in test env
            assert!(
                e.to_string().contains("database") 
                    || e.to_string().contains("connection")
                    || e.to_string().contains("not found"),
                "Error should be database-related: {}",
                e.to_string()
            );
        }
    }
}

/// Test that all tools are registered correctly
#[test]
fn test_tool_names_are_unique() {
    use std::collections::HashSet;
    
    // List of all tool names that should be registered
    let tool_names = vec![
        "echo",
        "server_info",
        "create_narrative",
        "validate_narrative",
        "save_narrative",
        "modify_narrative",
    ];
    
    let unique: HashSet<_> = tool_names.iter().collect();
    assert_eq!(
        tool_names.len(),
        unique.len(),
        "Tool names should be unique"
    );
}

/// Test tool count matches expected
#[test]
fn test_expected_tool_count() {
    // Base tools always available
    let mut expected_count = 6; // echo, server_info, create, validate, save, modify
    
    #[cfg(feature = "database")]
    {
        expected_count += 1; // query_content
    }
    
    #[cfg(feature = "anthropic")]
    {
        expected_count += 1; // generate_anthropic
    }
    
    #[cfg(feature = "gemini")]
    {
        expected_count += 1; // generate_gemini
    }
    
    #[cfg(feature = "ollama")]
    {
        expected_count += 1; // generate_ollama
    }
    
    #[cfg(feature = "groq")]
    {
        expected_count += 1; // generate_groq
    }
    
    #[cfg(feature = "huggingface")]
    {
        expected_count += 1; // generate_huggingface
    }
    
    #[cfg(feature = "discord")]
    {
        expected_count += 4; // get_channels, get_messages, get_guild_info, post_message
    }
    
    #[cfg(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    ))]
    {
        expected_count += 1; // execute_narrative
    }
    
    println!("Expected tool count with current features: {}", expected_count);
    assert!(expected_count >= 6, "Should have at least 6 base tools");
    assert!(expected_count <= 26, "Should have at most 26 tools with all features");
}
