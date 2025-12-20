//! Tool call result handler for MCP integration.

use botticelli_core::{ToolCall, ToolResult};
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use botticelli_mcp_client::UnifiedMcpClient;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

/// Handles tool call execution and result processing.
pub struct ToolCallHandler {
    mcp_client: Arc<RwLock<UnifiedMcpClient>>,
}

impl ToolCallHandler {
    /// Create a new tool call handler.
    #[instrument(skip(mcp_client))]
    pub fn new(mcp_client: Arc<RwLock<UnifiedMcpClient>>) -> Self {
        Self { mcp_client }
    }

    /// Execute a batch of tool calls and return results.
    #[instrument(skip(self, tool_calls))]
    pub async fn execute_tool_calls(
        &self,
        tool_calls: Vec<ToolCall>,
    ) -> ChatResult<Vec<ToolResult>> {
        if tool_calls.is_empty() {
            debug!("No tool calls to execute");
            return Ok(Vec::new());
        }

        info!(count = tool_calls.len(), "Executing tool calls");

        let mut results = Vec::with_capacity(tool_calls.len());

        for tool_call in tool_calls {
            let result = self.execute_single_tool_call(tool_call).await;
            results.push(result);
        }

        debug!(
            successful = results.iter().filter(|r| !r.is_error()).count(),
            failed = results.iter().filter(|r| *r.is_error()).count(),
            "Tool call execution complete"
        );

        Ok(results)
    }

    /// Execute a single tool call.
    #[instrument(skip(self))]
    async fn execute_single_tool_call(&self, tool_call: ToolCall) -> ToolResult {
        let tool_name = tool_call.name().to_string();
        let tool_id = tool_call.id().to_string();

        debug!(
            tool_name = %tool_name,
            tool_id = %tool_id,
            "Executing tool call"
        );

        // Get mutable lock on MCP client
        let mut client = self.mcp_client.write().await;

        // Execute via MCP client
        match client
            .execute_tool(&tool_name, tool_call.arguments().clone())
            .await
        {
            Ok(content) => {
                info!(
                    tool_name = %tool_name,
                    tool_id = %tool_id,
                    "Tool call succeeded"
                );
                ToolResult::new(tool_id, content, false)
            }
            Err(e) => {
                error!(
                    tool_name = %tool_name,
                    tool_id = %tool_id,
                    error = %e,
                    "Tool call failed"
                );
                let error_msg = serde_json::json!({
                    "error": format!("Tool execution failed: {}", e)
                });
                ToolResult::new(tool_id, error_msg, true)
            }
        }
    }

    /// Check if any tool call failed.
    #[instrument(skip(results))]
    pub fn has_errors(results: &[ToolResult]) -> bool {
        results.iter().any(|r| *r.is_error())
    }

    /// Get error messages from failed tool calls.
    #[instrument(skip(results))]
    pub fn get_error_messages(results: &[ToolResult]) -> Vec<String> {
        results
            .iter()
            .filter(|r| *r.is_error())
            .map(|r| format!("Tool {}: {:?}", r.tool_call_id(), r.content()))
            .collect()
    }

    /// Validate tool calls before execution.
    #[instrument(skip(tool_calls))]
    pub fn validate_tool_calls(tool_calls: &[ToolCall]) -> ChatResult<()> {
        if tool_calls.is_empty() {
            return Ok(());
        }

        // Check for duplicate IDs
        let mut seen_ids = std::collections::HashSet::new();
        for call in tool_calls {
            if !seen_ids.insert(call.id()) {
                warn!(tool_id = %call.id(), "Duplicate tool call ID detected");
                return Err(ChatError::new(ChatErrorKind::ValidationError(format!(
                    "Duplicate tool call ID: {}",
                    call.id()
                ))));
            }
        }

        // Validate tool names are not empty
        for call in tool_calls {
            if call.name().is_empty() {
                warn!("Tool call with empty name detected");
                return Err(ChatError::new(ChatErrorKind::ValidationError(
                    "Tool call name cannot be empty".to_string(),
                )));
            }
        }

        debug!(count = tool_calls.len(), "Tool calls validated");
        Ok(())
    }
}
