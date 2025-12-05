//! Server info tool.

use crate::tools::McpTool;
use crate::McpResult;
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::debug;

/// Tool that returns information about the Botticelli server.
pub struct ServerInfoTool;

#[async_trait]
impl McpTool for ServerInfoTool {
    fn name(&self) -> &str {
        "get_server_info"
    }

    fn description(&self) -> &str {
        "Returns information about the Botticelli MCP server, including version and capabilities."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: Value) -> McpResult<Value> {
        debug!("Server info tool called");

        Ok(json!({
            "name": "Botticelli MCP Server",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Model Context Protocol server for Botticelli LLM orchestration platform",
            "capabilities": {
                "tools": true,
                "resources": false,
                "prompts": false
            },
            "available_tools": [
                "echo",
                "get_server_info"
            ]
        }))
    }
}
