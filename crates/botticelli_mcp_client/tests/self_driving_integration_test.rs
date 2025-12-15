use botticelli_mcp_client::{ToolRegistry, ToolHandler};
use pmcp::{Content, ToolInfo};
use serde_json::json;
use std::sync::Arc;
use async_trait::async_trait;

/// Simple test tool implementation
struct TestTool {
    name: String,
    description: String,
}

#[async_trait]
impl ToolHandler for TestTool {
    async fn execute(&self, args: serde_json::Value) -> botticelli_mcp_client::McpClientResult<Vec<Content>> {
        Ok(vec![Content::Text {
            text: format!("Executed {} with args: {}", self.name, args)
        }])
    }

    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            &self.name,
            Some(self.description.clone()),
            json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            }),
        )
    }
}

/// Test that the tool registry can be created
#[tokio::test]
async fn test_registry_initialization() {
    let registry = ToolRegistry::new();
    assert_eq!(registry.list_tools().len(), 0, "New registry should be empty");
}

/// Test tool registration
#[tokio::test]
async fn test_tool_registration() {
    let mut registry = ToolRegistry::new();
    
    let tool = Arc::new(TestTool {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
    });
    
    registry.register("test_tool".to_string(), tool)
        .expect("Registration should succeed");
    
    let tools = registry.list_tools();
    assert_eq!(tools.len(), 1, "Should have one tool");
    assert_eq!(tools[0].name, "test_tool");
}

/// Test tool execution
#[tokio::test]
async fn test_tool_execution() {
    let mut registry = ToolRegistry::new();
    
    let tool = Arc::new(TestTool {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
    });
    
    registry.register("test_tool".to_string(), tool)
        .expect("Registration should succeed");
    
    let result = registry.execute_tool("test_tool", json!({"input": "hello"}))
        .await
        .expect("Tool execution should succeed");
    
    assert_eq!(result.len(), 1, "Should have one content item");
}

/// Test error handling for invalid tool calls
#[tokio::test]
async fn test_invalid_tool_call() {
    let registry = ToolRegistry::new();
    
    // Call non-existent tool
    let result = registry.execute_tool("nonexistent_tool", json!({})).await;
    
    assert!(result.is_err(), "Should error for non-existent tool");
}

/// Test duplicate registration prevention
#[tokio::test]
async fn test_duplicate_registration() {
    let mut registry = ToolRegistry::new();
    
    let tool = Arc::new(TestTool {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
    });
    
    // First registration should succeed
    registry.register("test_tool".to_string(), Arc::clone(&tool) as Arc<dyn ToolHandler>)
        .expect("First registration should succeed");
    
    // Second registration should fail
    let result = registry.register("test_tool".to_string(), tool as Arc<dyn ToolHandler>);
    assert!(result.is_err(), "Duplicate registration should fail");
}

/// Test concurrent tool execution
#[tokio::test]
async fn test_concurrent_execution() {
    let mut registry = ToolRegistry::new();
    
    let tool = Arc::new(TestTool {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
    });
    
    registry.register("test_tool".to_string(), tool)
        .expect("Registration should succeed");
    
    let registry = Arc::new(registry);

    // Spawn multiple concurrent requests
    let mut handles = vec![];
    for _ in 0..10 {
        let registry = Arc::clone(&registry);
        let handle = tokio::spawn(async move {
            registry.execute_tool("test_tool", json!({"input": "test"})).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let result = handle.await.expect("Task should complete")
            .expect("Request should succeed");
        assert!(!result.is_empty(), "Should have content");
    }
}

/// Test registry lifecycle
#[tokio::test]
async fn test_registry_lifecycle() {
    // Create registry
    let mut registry = ToolRegistry::new();
    
    let tool = Arc::new(TestTool {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
    });
    
    registry.register("test_tool".to_string(), tool)
        .expect("Registration should succeed");

    // Verify it's usable
    let tools = registry.list_tools();
    assert_eq!(tools.len(), 1);

    // Registry should be droppable without issues
    drop(registry);
}

/// Test tool schema validation
#[tokio::test]
async fn test_tool_schemas() {
    let mut registry = ToolRegistry::new();
    
    let tool = Arc::new(TestTool {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
    });
    
    registry.register("test_tool".to_string(), tool)
        .expect("Registration should succeed");

    let tools = registry.list_tools();
    
    for tool in tools {
        // Each tool should have a valid JSON schema
        assert!(tool.input_schema.is_object(), 
            "Tool {} should have object schema", tool.name);
        
        // Schema should have required fields
        let schema = tool.input_schema.as_object().unwrap();
        assert!(schema.contains_key("type"), 
            "Tool {} schema should have type", tool.name);
    }
}

/// Test multiple tool registration
#[tokio::test]
async fn test_multiple_tools() {
    let mut registry = ToolRegistry::new();
    
    for i in 0..5 {
        let tool = Arc::new(TestTool {
            name: format!("tool_{}", i),
            description: format!("Test tool {}", i),
        });
        
        registry.register(format!("tool_{}", i), tool)
            .expect("Registration should succeed");
    }
    
    let tools = registry.list_tools();
    assert_eq!(tools.len(), 5, "Should have 5 tools");
}
