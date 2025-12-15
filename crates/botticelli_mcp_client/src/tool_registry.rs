use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use async_trait::async_trait;
use pmcp::{Content, ToolInfo};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Handler for executing tool calls
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with given arguments
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>>;

    /// Get tool metadata
    fn tool_info(&self) -> ToolInfo;
}

/// Registry for managing internal Botticelli tools
#[derive(Clone)]
pub struct ToolRegistry {
    /// Registered tool handlers
    handlers: Arc<HashMap<String, Arc<dyn ToolHandler>>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    #[instrument]
    pub fn new() -> Self {
        debug!("Creating new tool registry");
        Self {
            handlers: Arc::new(HashMap::new()),
        }
    }

    /// Register a tool handler
    #[instrument(skip(self, handler))]
    pub fn register(&mut self, name: String, handler: Arc<dyn ToolHandler>) -> McpClientResult<()> {
        debug!(tool_name = %name, "Registering tool");
        
        let handlers = Arc::make_mut(&mut self.handlers);
        if handlers.contains_key(&name) {
            return Err(McpClientError::new(
                McpClientErrorKind::InvalidToolCall(format!("Tool already registered: {}", name))
            ));
        }
        
        handlers.insert(name, handler);
        Ok(())
    }

    /// Get all registered tools
    #[instrument(skip(self))]
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        debug!("Listing all registered tools");
        self.handlers
            .values()
            .map(|handler| handler.tool_info())
            .collect()
    }

    /// Execute a tool by name
    #[instrument(skip(self, arguments))]
    pub async fn execute_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> McpClientResult<Vec<Content>> {
        debug!(tool_name = %name, "Executing tool");
        
        let handler = self
            .handlers
            .get(name)
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::ToolNotFound(name.to_string()))
            })?;

        handler.execute(arguments).await
    }

    /// Check if a tool is registered
    #[instrument(skip(self))]
    pub fn has_tool(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }

    /// Get the number of registered tools
    pub fn tool_count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTool;

    #[async_trait]
    impl ToolHandler for TestTool {
        async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
            Ok(vec![Content::Text {
                text: format!("Executed with: {}", args),
            }])
        }

        fn tool_info(&self) -> ToolInfo {
            ToolInfo::new(
                "test_tool",
                Some("A test tool".to_string()),
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                }),
            )
        }
    }

    #[tokio::test]
    async fn test_register_and_execute() {
        let mut registry = ToolRegistry::new();
        let handler = Arc::new(TestTool);

        registry
            .register("test_tool".to_string(), handler)
            .expect("Registration failed");

        assert!(registry.has_tool("test_tool"));
        assert_eq!(registry.tool_count(), 1);

        let result = registry
            .execute_tool("test_tool", serde_json::json!({"key": "value"}))
            .await
            .expect("Execution failed");

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_list_tools() {
        let mut registry = ToolRegistry::new();
        let handler = Arc::new(TestTool);

        registry
            .register("test_tool".to_string(), handler)
            .expect("Registration failed");

        let tools = registry.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let mut registry = ToolRegistry::new();
        let handler = Arc::new(TestTool);

        registry
            .register("test_tool".to_string(), handler.clone())
            .expect("First registration failed");

        let result = registry.register("test_tool".to_string(), handler);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let registry = ToolRegistry::new();
        let result = registry
            .execute_tool("unknown", serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }
}
