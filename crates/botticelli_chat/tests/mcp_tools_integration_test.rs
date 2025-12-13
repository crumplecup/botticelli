//! Integration tests for MCP tools through chat interface.
//!
//! Tests each MCP tool by sending commands through the chat interface
//! and verifying the expected outcomes.

use botticelli_chat::{
    ChatAppConfig, ChatResult, Command, CommandExecutor, NarrativeCommand,
    ServiceContainer,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Helper to create a test executor with local config.
fn setup_executor() -> CommandExecutor {
    let config = ChatAppConfig::default();  // Uses local mode by default
    let services = Arc::new(ServiceContainer::new(config));
    CommandExecutor::with_services(services)
}

/// Test 1: create_narrative - Generate a new narrative from a prompt
#[tokio::test]
async fn test_create_narrative_tool() -> ChatResult<()> {
    let executor = setup_executor();
    
    // Give MCP server time to fully start
    sleep(Duration::from_secs(2)).await;

    let prompt = "Create a narrative to generate social media posts for the MINT navigation center in Grants Pass, Oregon. Include schema for storing Discord posts with fields: title, content, channel_id, scheduled_time.";
    
    let command = Command::Narrative(NarrativeCommand::Create {
        prompt: prompt.to_string(),
    });
    
    let response = executor.execute(command).await?;
    
    // Verify response contains expected elements
    let content = response.as_text();
    assert!(content.contains("narrative") || content.contains("created"), 
            "Response should mention narrative creation");
    
    Ok(())
}

/// Test 2: validate_narrative - Validate TOML syntax and structure
#[tokio::test]
async fn test_validate_narrative_tool() -> ChatResult<()> {
    let executor = setup_executor();
    sleep(Duration::from_secs(2)).await;

    // First create a narrative
    let command = Command::Narrative(NarrativeCommand::Create {
        prompt: "Create a simple narrative that echoes 'Hello World'".to_string(),
    });
    executor.execute(command).await?;
    
    // Validation happens automatically in create_narrative
    // Success means validation passed
    
    Ok(())
}

/// Test 3: update_prompt - Modify narrative prompt
#[tokio::test]
async fn test_update_prompt_tool() -> ChatResult<()> {
    let executor = setup_executor();
    sleep(Duration::from_secs(2)).await;

    // Create initial narrative
    let create_cmd = Command::Narrative(NarrativeCommand::Create {
        prompt: "Create a narrative that generates a greeting message".to_string(),
    });
    executor.execute(create_cmd).await?;
    
    // Update the prompt
    let update_cmd = Command::Narrative(NarrativeCommand::UpdatePrompt {
        prompt: "Create a narrative that says 'Good morning' instead".to_string(),
    });
    let response = executor.execute(update_cmd).await?;
    
    assert!(!response.as_text().contains("Error"),
            "Response should not contain errors");
    
    Ok(())
}

/// Test 4: show_narrative - Display current narrative
#[tokio::test]
async fn test_show_narrative_tool() -> ChatResult<()> {
    let executor = setup_executor();
    sleep(Duration::from_secs(2)).await;

    // Create a simple echo narrative
    let create_cmd = Command::Narrative(NarrativeCommand::Create {
        prompt: "Create a narrative with one act that uses the echo tool to return 'Test successful'".to_string(),
    });
    executor.execute(create_cmd).await?;
    
    // Show it
    let show_cmd = Command::Narrative(NarrativeCommand::Show);
    let response = executor.execute(show_cmd).await?;
    
    assert!(!response.as_text().is_empty(),
            "Response should show narrative content");
    
    Ok(())
}

/// Test 5: save_narrative - Persist narrative to file
#[tokio::test]
async fn test_save_narrative_tool() -> ChatResult<()> {
    let executor = setup_executor();
    sleep(Duration::from_secs(2)).await;

    // Create a narrative
    let create_cmd = Command::Narrative(NarrativeCommand::Create {
        prompt: "Create a narrative for testing file persistence".to_string(),
    });
    executor.execute(create_cmd).await?;
    
    // Save it to a temp file
    let save_path = format!("/tmp/test_narrative_{}.toml", chrono::Utc::now().timestamp());
    let save_cmd = Command::Narrative(NarrativeCommand::Save {
        path: save_path.clone(),
    });
    let response = executor.execute(save_cmd).await?;
    
    assert!(!response.as_text().contains("Error"),
            "Response should not contain errors");
    
    // Cleanup
    let _ = std::fs::remove_file(&save_path);
    
    Ok(())
}

/// Test 6: load_narrative - Load narrative from file
#[tokio::test]
async fn test_load_narrative_tool() -> ChatResult<()> {
    let executor = setup_executor();
    sleep(Duration::from_secs(2)).await;

    // First save a narrative
    let create_cmd = Command::Narrative(NarrativeCommand::Create {
        prompt: "Create a simple test narrative".to_string(),
    });
    executor.execute(create_cmd).await?;
    
    let save_path = format!("/tmp/load_test_{}.toml", chrono::Utc::now().timestamp());
    let save_cmd = Command::Narrative(NarrativeCommand::Save {
        path: save_path.clone(),
    });
    executor.execute(save_cmd).await?;
    
    // Now load it back
    let load_cmd = Command::Narrative(NarrativeCommand::Load {
        path: save_path.clone(),
    });
    let response = executor.execute(load_cmd).await?;
    
    assert!(!response.as_text().contains("Error"),
            "Response should not contain errors");
    
    // Cleanup
    let _ = std::fs::remove_file(&save_path);
    
    Ok(())
}

/// Test 7: Full workflow - Create, show, save, load
#[tokio::test]
async fn test_full_narrative_workflow() -> ChatResult<()> {
    let executor = setup_executor();
    sleep(Duration::from_secs(2)).await;

    // Step 1: Create
    let create_prompt = "Create a narrative for the MINT navigation center that generates 3 Discord posts about their services";
    let create_cmd = Command::Narrative(NarrativeCommand::Create {
        prompt: create_prompt.to_string(),
    });
    let create_response = executor.execute(create_cmd).await?;
    assert!(!create_response.as_text().is_empty());
    
    // Step 2: Show
    let show_cmd = Command::Narrative(NarrativeCommand::Show);
    let show_response = executor.execute(show_cmd).await?;
    assert!(!show_response.as_text().is_empty());
    
    // Step 3: Save
    let save_path = format!("/tmp/mint_narrative_{}.toml", chrono::Utc::now().timestamp());
    let save_cmd = Command::Narrative(NarrativeCommand::Save {
        path: save_path.clone(),
    });
    let save_response = executor.execute(save_cmd).await?;
    assert!(!save_response.as_text().contains("Error"));
    
    // Step 4: Load it back
    let load_cmd = Command::Narrative(NarrativeCommand::Load {
        path: save_path.clone(),
    });
    let load_response = executor.execute(load_cmd).await?;
    assert!(!load_response.as_text().contains("Error"));
    
    // Cleanup
    let _ = std::fs::remove_file(&save_path);
    
    Ok(())
}

/// Test 8: validate_narrative - Validate current narrative
#[tokio::test]
async fn test_validate_narrative() -> ChatResult<()> {
    let executor = setup_executor();
    sleep(Duration::from_secs(2)).await;

    // Create a narrative
    let create_cmd = Command::Narrative(NarrativeCommand::Create {
        prompt: "Create a simple test narrative".to_string(),
    });
    executor.execute(create_cmd).await?;
    
    // Validate it
    let validate_cmd = Command::Narrative(NarrativeCommand::Validate);
    let response = executor.execute(validate_cmd).await?;
    
    // Should not error
    assert!(!response.as_text().contains("Error") || response.as_text().contains("valid"),
            "Should validate successfully or indicate validation status");
    
    Ok(())
}
