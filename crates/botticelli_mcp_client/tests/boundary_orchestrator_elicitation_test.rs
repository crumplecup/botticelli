//! Boundary Test: MCP Orchestrator ↔ Elicitation System
//!
//! Tests the handoff between orchestrator and elicitation tools.
//! Verifies partial narrative workflows and carousel generation.

use botticelli_mcp_client::{UnifiedMcpClient, register_internal_tools};
use serde_json::json;

#[tokio::test]
async fn test_orchestrator_to_elicitation_boundary() {
    // Setup
    let mut client = UnifiedMcpClient::builder().build();
    register_internal_tools(client.internal_registry_mut(), "/tmp/narratives")
        .expect("Register tools");

    // Execute: Start elicitation workflow
    let params = json!({
        "description": "A fantasy adventure"
    });

    let result = client
        .execute_tool("create_elicitation_session", params)
        .await
        .expect("Elicitation session should start");

    // Verify: Session ID is returned
    // Result is an array of content items: [{ "text": "..." }]
    let text = result[0]["text"]
        .as_str()
        .expect("Should have text content");
    let response: serde_json::Value = serde_json::from_str(text).expect("Should be valid JSON");
    assert!(
        response["session_id"].is_string(),
        "Should return session ID"
    );
}

#[tokio::test]
async fn test_carousel_generation_boundary() {
    // Setup
    let mut client = UnifiedMcpClient::builder().build();
    register_internal_tools(client.internal_registry_mut(), "/tmp/narratives")
        .expect("Register tools");

    // First create a session
    let session_params = json!({
        "description": "A fantasy adventure"
    });

    let session_result = client
        .execute_tool("create_elicitation_session", session_params)
        .await
        .expect("Session should be created");

    let text = session_result[0]["text"]
        .as_str()
        .expect("Should have text content");
    let response: serde_json::Value = serde_json::from_str(text).expect("Should be valid JSON");
    let session_id = response["session_id"]
        .as_str()
        .expect("Should have session_id")
        .to_string();

    // Execute: Generate carousel
    let params = json!({
        "iterations": 3,
        "estimated_tokens_per_iteration": 500,
        "continue_on_error": false,
        "narrative_template": "test_template.toml"
    });

    let result = client
        .execute_tool("create_carousel", params)
        .await
        .expect("Carousel creation should succeed");

    // Verify: Carousel ID is returned in text
    let text = result[0]["text"].as_str().expect("Should have text");
    assert!(
        text.contains("Created carousel with ID"),
        "Should return carousel info"
    );
}

#[tokio::test]
async fn test_elicitation_state_transition_boundary() {
    // Setup
    let mut client = UnifiedMcpClient::builder().build();
    register_internal_tools(client.internal_registry_mut(), "/tmp/narratives")
        .expect("Register tools");

    // Start elicitation session
    let start_params = json!({
        "description": "A fantasy adventure"
    });
    let session_result = client
        .execute_tool("create_elicitation_session", start_params)
        .await
        .expect("Should start");

    let text = session_result[0]["text"]
        .as_str()
        .expect("Should have text content");
    let response: serde_json::Value = serde_json::from_str(text).expect("Should be valid JSON");
    let session_id = response["session_id"]
        .as_str()
        .expect("Should have session_id")
        .to_string();

    // Elicit metadata
    let metadata_params = json!({
        "session_id": session_id,
        "title": "The Quest",
        "theme": "Adventure"
    });

    let result = client
        .execute_tool("elicit_metadata", metadata_params)
        .await;

    // Verify: Metadata update succeeds
    assert!(result.is_ok(), "Metadata elicitation should succeed");
}
