//! Adapters for wrapping existing McpTool implementations as pmcp ToolHandlers.
//!
//! This module provides a bridge between our existing tool implementations
//! and the pmcp SDK's ToolHandler trait.

use crate::tools::McpTool;
use async_trait::async_trait;
use pmcp::{RequestHandlerExtra, ToolHandler};
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, instrument};

/// Adapter that wraps a McpTool to implement pmcp's ToolHandler trait.
///
/// This allows us to reuse all our existing tool implementations
/// without rewriting them.
pub struct McpToolAdapter<T: McpTool> {
    tool: Arc<T>,
}

impl<T: McpTool> McpToolAdapter<T> {
    /// Creates a new adapter wrapping the given tool.
    pub fn new(tool: T) -> Self {
        Self {
            tool: Arc::new(tool),
        }
    }
}

#[async_trait]
impl<T: McpTool + 'static> ToolHandler for McpToolAdapter<T> {
    #[instrument(skip(self, _extra), fields(tool_name = self.tool.name()))]
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> pmcp::Result<Value> {
        // Execute the tool using our existing McpTool trait
        match self.tool.execute(args).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!(error = ?e, "Tool execution failed");
                // Convert our error to pmcp error
                Err(pmcp::Error::internal(e.to_string()))
            }
        }
    }
}

/// Type-erased adapter for Arc<dyn McpTool>.
///
/// This allows registering tools from a ToolRegistry that returns trait objects.
pub struct DynMcpToolAdapter {
    tool: Arc<dyn McpTool>,
}

impl DynMcpToolAdapter {
    /// Creates a new adapter from an Arc<dyn McpTool>.
    pub fn new(tool: Arc<dyn McpTool>) -> Self {
        Self { tool }
    }
}

#[async_trait]
impl ToolHandler for DynMcpToolAdapter {
    #[instrument(skip(self, _extra), fields(tool_name = self.tool.name()))]
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> pmcp::Result<Value> {
        match self.tool.execute(args).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!(error = ?e, "Tool execution failed");
                Err(pmcp::Error::internal(e.to_string()))
            }
        }
    }
}
