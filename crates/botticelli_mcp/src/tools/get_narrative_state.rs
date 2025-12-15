use crate::tools::{
    GetNarrativeStateInput, GetNarrativeStateOutput, McpTool, PartialNarrativeRegistry,
};
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use serde_json::{json, Value};
use std::sync::Arc;

/// Tool for getting the current state of a narrative elicitation session.
pub struct GetNarrativeStateTool {
    registry: Arc<PartialNarrativeRegistry>,
}

impl GetNarrativeStateTool {
    /// Creates a new get narrative state tool.
    pub fn new(registry: Arc<PartialNarrativeRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl McpTool for GetNarrativeStateTool {
    fn name(&self) -> &str {
        "get_narrative_state"
    }

    fn description(&self) -> &str {
        "Get the current state and completeness of a narrative elicitation session. \
         Returns summary information including acts, completeness percentage, and optionally TOML representation."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {
                    "type": "string",
                    "description": "UUID of the narrative session"
                },
                "format": {
                    "type": "string",
                    "enum": ["summary", "full", "toml"],
                    "description": "Output format (default: summary)",
                    "default": "summary"
                }
            },
            "required": ["narrative_id"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let input: GetNarrativeStateInput =
            serde_json::from_value(input).map_err(|e| McpError::invalid_input(e.to_string()))?;
        let output =
            crate::tools::elicitation::state::get_narrative_state(self.registry.as_ref(), input)
                .await?;
        serde_json::to_value(output).map_err(|e| McpError::execution_failed(e.to_string()))
    }
}
