//! Boundary Test: MCP Orchestrator ↔ Elicitation System
//!
//! Tests the handoff between orchestrator and elicitation tools.
//! Verifies partial narrative workflows and carousel generation.

use botticelli_mcp_client::{ToolRegistry, UnifiedMcpClient};
use serde_json::json;

#[tokio::test]
async fn test_orchestrator_to_elicitation_boundary() {
    // Setup
    let registry = ToolRegistry::new();
    registry.register_elicitation_tools().await;
    let client = UnifiedMcpClient::new();

    // Execute: Start elicitation workflow
    let params = json!({
        "actor_id": "test_actor",
        "elicitation_type": "partial_narrative"
    });

    let result = client
        .call_tool("start_elicitation", params)
        .await
        .expect("Elicitation should start");

    // Verify: Partial narrative is created
    assert!(
        result.contains_key("partial_narrative_id"),
        "Should return partial narrative ID"
    );
}

#[tokio::test]
async fn test_carousel_generation_boundary() {
    // Setup
    let registry = ToolRegistry::new();
    registry.register_elicitation_tools().await;
    let client = UnifiedMcpClient::new();

    // Execute: Generate carousel
    let params = json!({
        "partial_narrative_id": "test_partial",
        "options": ["Option A", "Option B", "Option C"]
    });

    let result = client
        .call_tool("create_carousel", params)
        .await
        .expect("Carousel creation should succeed");

    // Verify: Carousel structure is valid
    assert!(result.is_object(), "Should return carousel data");
    assert!(result["options"].is_array(), "Should have options array");
}

#[tokio::test]
async fn test_elicitation_state_transition_boundary() {
    // Setup
    let registry = ToolRegistry::new();
    registry.register_elicitation_tools().await;
    let client = UnifiedMcpClient::new();

    // Start elicitation
    let start_params = json!({
        "actor_id": "test_actor",
        "elicitation_type": "partial_narrative"
    });
    let partial = client
        .call_tool("start_elicitation", start_params)
        .await
        .expect("Should start");

    // Update state
    let update_params = json!({
        "partial_narrative_id": partial["partial_narrative_id"],
        "field": "setting",
        "value": "A dark forest"
    });

    let result = client
        .call_tool("update_partial_narrative", update_params)
        .await;

    // Verify: State update propagates
    assert!(result.is_ok(), "State update should succeed");
}
