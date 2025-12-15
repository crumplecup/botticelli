//! Boundary Test: MCP Server ↔ Internal Tool Executor
//!
//! Tests the handoff between MCP protocol layer and internal tool execution.
//! Verifies tool registration, listing, and execution work correctly.

use botticelli_mcp_client::{InternalToolExecutor, ToolRegistry};
use serde_json::json;

#[tokio::test]
async fn test_tool_registration_boundary() {
    // Setup
    let registry = ToolRegistry::new();
    let executor = InternalToolExecutor::new(registry.clone());

    // Register a test tool
    registry.register_narrative_tool().await;

    // Verify: Tool appears in registry
    let tools = registry.list_tools().await;
    assert!(
        tools.iter().any(|t| t.name == "generate_narrative"),
        "Narrative tool should be registered"
    );
}

#[tokio::test]
async fn test_tool_execution_boundary() {
    // Setup
    let registry = ToolRegistry::new();
    let executor = InternalToolExecutor::new(registry.clone());
    registry.register_narrative_tool().await;

    // Execute: Call tool through executor
    let params = json!({
        "actor_id": "test_actor",
        "prompt": "Test narrative generation"
    });

    let result = executor
        .execute("generate_narrative", params)
        .await
        .expect("Tool execution should succeed");

    // Verify: Result has expected structure
    assert!(result.is_object(), "Tool result should be a JSON object");
}

#[tokio::test]
async fn test_tool_error_handling_boundary() {
    // Setup
    let registry = ToolRegistry::new();
    let executor = InternalToolExecutor::new(registry.clone());
    registry.register_narrative_tool().await;

    // Execute: Call with invalid params
    let invalid_params = json!({
        "invalid_field": "bad_value"
    });

    let result = executor.execute("generate_narrative", invalid_params).await;

    // Verify: Error is properly propagated
    assert!(result.is_err(), "Invalid params should return error");
}

#[tokio::test]
async fn test_unknown_tool_boundary() {
    // Setup
    let registry = ToolRegistry::new();
    let executor = InternalToolExecutor::new(registry);

    // Execute: Call non-existent tool
    let result = executor.execute("nonexistent_tool", json!({})).await;

    // Verify: Error indicates unknown tool
    assert!(result.is_err(), "Unknown tool should return error");
}
