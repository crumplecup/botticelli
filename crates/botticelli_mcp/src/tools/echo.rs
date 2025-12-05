//! Echo tool for testing MCP server.

use crate::tools::McpTool;
use crate::{McpError, McpResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::debug;

/// Simple echo tool that returns its input.
pub struct EchoTool;

#[async_trait]
impl McpTool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes back the input message. Useful for testing the MCP connection."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo back"
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        debug!(input = ?input, "Echo tool called");

        let message = input
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'message' field".to_string()))?;

        Ok(json!({
            "echo": message,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}
