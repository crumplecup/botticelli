//! Discord content workflow orchestration.

use crate::ToolRegistry;
use botticelli_error::{McpError, McpResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{debug, error, info, instrument};

/// Tool for end-to-end Discord content workflows.
///
/// Orchestrates narrative execution and Discord posting in a single operation.
#[derive(Clone)]
pub struct DiscordContentWorkflowTool {
    registry: ToolRegistry,
}

impl DiscordContentWorkflowTool {
    /// Creates a new Discord content workflow tool.
    #[instrument(skip(registry))]
    pub fn new(registry: ToolRegistry) -> Self {
        debug!("Creating Discord content workflow tool");
        Self { registry }
    }

    /// Executes a workflow: narrative execution → content extraction → Discord posting.
    #[instrument(skip(self))]
    pub async fn execute_workflow(
        &self,
        narrative_path: PathBuf,
        channel_id: String,
        variables: Option<Value>,
    ) -> McpResult<Value> {
        info!(
            narrative = %narrative_path.display(),
            channel = %channel_id,
            "Starting Discord content workflow"
        );

        // Step 1: Execute narrative
        debug!("Executing narrative");
        let execute_tool = self
            .registry
            .get("execute_narrative")
            .ok_or_else(|| McpError::tool_not_found("execute_narrative".to_string()))?;
        
        let execution_result = execute_tool
            .execute(json!({
                "narrative_path": narrative_path.to_string_lossy(),
                "variables": variables.unwrap_or_else(|| json!({}))
            }))
            .await?;

        debug!(result = ?execution_result, "Narrative executed");

        // Step 2: Extract content from narrative output
        let content = self.extract_content(&execution_result)?;
        debug!(content = %content, "Content extracted");

        // Step 3: Post to Discord
        let post_tool = self
            .registry
            .get("discord_post_message")
            .ok_or_else(|| McpError::tool_not_found("discord_post_message".to_string()))?;
        
        let post_result = post_tool
            .execute(json!({
                "channel_id": channel_id.clone(),
                "content": content.clone()
            }))
            .await?;

        info!(
            channel = %channel_id,
            message_id = ?post_result.get("id"),
            "Content posted to Discord"
        );

        // Step 4: Return combined result
        Ok(json!({
            "workflow": "discord_content",
            "narrative": narrative_path.to_string_lossy(),
            "channel_id": channel_id,
            "execution": execution_result,
            "discord_post": post_result,
            "status": "success"
        }))
    }

    /// Extracts content from narrative execution result.
    #[instrument(skip(self, result))]
    fn extract_content(&self, result: &Value) -> McpResult<String> {
        debug!("Extracting content from result");

        // Try multiple extraction strategies
        if let Some(content) = result.get("content").and_then(Value::as_str) {
            return Ok(content.to_string());
        }

        if let Some(output) = result.get("output").and_then(Value::as_str) {
            return Ok(output.to_string());
        }

        if let Some(text) = result.get("text").and_then(Value::as_str) {
            return Ok(text.to_string());
        }

        // Fallback: stringify the entire result
        error!("No standard content field found, using full result");
        Ok(serde_json::to_string_pretty(result)
            .unwrap_or_else(|_| "Failed to extract content".to_string()))
    }

    /// MCP tool handler for workflow execution.
    #[instrument(skip(self))]
    pub async fn handle(&self, params: Value) -> McpResult<Value> {
        let narrative_path = params
            .get("narrative_path")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::invalid_input("Missing narrative_path parameter".to_string()))?;

        let channel_id = params
            .get("channel_id")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::invalid_input("Missing channel_id parameter".to_string()))?;

        let variables = params.get("variables").cloned();

        self.execute_workflow(PathBuf::from(narrative_path), channel_id.to_string(), variables)
            .await
    }
}
