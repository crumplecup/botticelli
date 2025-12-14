#![cfg(feature = "cli")]

//! End-to-end integration test for narrative creation workflow
//!
//! Tests the complete flow:
//! 1. MCP server is running and accessible
//! 2. MCP server has create_narrative tool
//! 3. Can call create_narrative successfully
//! 4. Can save narrative to database
//! 5. Can load narrative from database

use botticelli_chat::ChatAppConfig;
#[cfg(feature = "cli")]
use botticelli_chat::startup_sequence;
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use std::sync::Once;

static INIT: Once = Once::new();

/// Initializes test environment - runs startup sequence once
async fn init_test_environment() -> ChatResult<ChatAppConfig> {
    let config = ChatAppConfig::load(None).map_err(|e| {
        ChatError::new(ChatErrorKind::IoError(format!("Failed to load config: {}", e)))
    })?;
    
    INIT.call_once(|| {
        // This will only run once across all tests
    });
    
    // Run startup sequence to ensure services are ready
    #[cfg(feature = "cli")]
    startup_sequence(&config).await?;
    
    Ok(config)
}

/// Test that MCP server is accessible
#[tokio::test]
async fn test_mcp_server_accessible() -> ChatResult<()> {
    let config = init_test_environment().await?;
    let mcp_config = &config.mcp_server;

    let base_url = format!("http://{}:{}", mcp_config.host, mcp_config.port);
    let client = reqwest::Client::new();
    
    // Test health endpoint
    let response = client
        .get(&format!("{}/health", base_url))
        .send()
        .await
        .map_err(|e| ChatError::new(ChatErrorKind::IoError(
            format!("MCP server health check failed at {}:{}. Error: {}", 
                mcp_config.host, mcp_config.port, e)
        )))?;
    
    assert!(
        response.status().is_success(),
        "MCP server health check returned: {}",
        response.status()
    );
    
    println!("✅ MCP server is accessible at {}", base_url);
    Ok(())
}

/// Test that MCP server has create_narrative tool
#[tokio::test]
async fn test_mcp_server_has_create_narrative_tool() -> ChatResult<()> {
    let config = init_test_environment().await?;
    let mcp_config = &config.mcp_server;

    let base_url = format!("http://{}:{}", mcp_config.host, mcp_config.port);
    let client = reqwest::Client::new();
    
    // Test tools list endpoint
    let response = client
        .get(&format!("{}/tools/list", base_url))
        .send()
        .await
        .map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Failed to list MCP tools: {}. Is the MCP server running?",
                e
            )))
        })?;
    
    assert!(
        response.status().is_success(),
        "MCP tools list returned: {}",
        response.status()
    );
    
    let tools: serde_json::Value = response
        .json()
        .await
        .map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!("Failed to parse tools response: {}", e)))
        })?;
    
    // Check that create_narrative tool exists
    let has_create_narrative = tools
        .get("tools")
        .and_then(|t| t.as_array())
        .map(|tools| {
            tools.iter().any(|tool| {
                tool.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n == "create_narrative")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    
    assert!(
        has_create_narrative,
        "MCP server does not have create_narrative tool. Available tools: {}",
        tools
    );
    
    println!("✅ MCP server has create_narrative tool");
    Ok(())
}

/// Test MCP server create_narrative call
#[tokio::test]
async fn test_create_narrative_call() -> ChatResult<()> {
    let config = init_test_environment().await?;
    let mcp_config = &config.mcp_server;

    let base_url = format!("http://{}:{}", mcp_config.host, mcp_config.port);
    let client = reqwest::Client::new();
    
    // Build request payload
    let payload = serde_json::json!({
        "name": "create_narrative",
        "parameters": {
            "name": "test_explorer_narrative",
            "description": "A test narrative about a brave explorer"
        }
    });
    
    // Call create_narrative tool
    let response = client
        .post(&format!("{}/tools/call", base_url))
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Failed to call create_narrative: {}. Is the MCP server running?",
                e
            )))
        })?;
    
    // Get response body for debugging
    let status = response.status();
    let body_text = response.text().await.map_err(|e| {
        ChatError::new(ChatErrorKind::IoError(format!("Failed to read response: {}", e)))
    })?;
    
    assert!(
        status.is_success(),
        "create_narrative call returned: {} - Body: {}",
        status,
        body_text
    );
    
    let result: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
        ChatError::new(ChatErrorKind::IoError(format!(
            "Failed to parse create_narrative response: {}",
            e
        )))
    })?;
    
    // Verify response has content
    // Get the content text (which is a JSON string containing the narrative result)
    let content_text = result
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Response missing 'content[0].text' field. Got: {}",
                result
            )))
        })?;
    
    // Parse the JSON response from create_narrative
    let narrative_result: serde_json::Value = serde_json::from_str(content_text).map_err(|e| {
        ChatError::new(ChatErrorKind::ValidationError(format!(
            "Failed to parse narrative result: {}",
            e
        )))
    })?;
    
    // Verify the narrative was created successfully
    let toml_content = narrative_result
        .get("toml")
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            ChatError::new(ChatErrorKind::IoError(
                "Response missing 'toml' field".to_string(),
            ))
        })?;
    
    assert!(
        !toml_content.is_empty(),
        "MCP returned empty TOML content"
    );
    
    // Verify TOML is valid
    let _parsed: toml::Value = toml::from_str(toml_content).map_err(|e| {
        ChatError::new(ChatErrorKind::ValidationError(format!(
            "Invalid TOML generated: {}",
            e
        )))
    })?;
    
    println!("✅ MCP server successfully created valid narrative");
    println!("TOML preview: {}", &toml_content[..toml_content.len().min(200)]);
    
    // Verify expected sections exist
    assert!(
        toml_content.contains("[narrative]"),
        "TOML missing [narrative] section"
    );
    assert!(toml_content.contains("[toc]"), "TOML missing [toc] section");
    assert!(toml_content.contains("[acts]"), "TOML missing [acts] section");
    
    Ok(())
}
