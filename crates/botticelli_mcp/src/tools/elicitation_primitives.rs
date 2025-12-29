//! Primitive elicitation tools for the elicitation crate paradigm system.
//!
//! These tools provide the basic building blocks (`elicit_select`,
//! `elicit_number`, `elicit_bool`) that the elicitation crate's derive macros
//! expect when calling MCP tools.

use crate::dialog_resource::DialogResource;
use crate::tools::{McpResult, McpTool};
use async_trait::async_trait;
use botticelli_error::McpError;
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::instrument;

/// Tool for selecting one option from a finite list.
///
/// This tool corresponds to the `elicit_select` MCP tool expected by
/// the elicitation crate for Select paradigm types (enums).
pub struct ElicitSelectTool {
    dialog: Arc<DialogResource>,
}

impl ElicitSelectTool {
    /// Create a new select elicitation tool.
    pub fn new(dialog: Arc<DialogResource>) -> Self {
        Self { dialog }
    }
}

#[async_trait]
impl McpTool for ElicitSelectTool {
    fn name(&self) -> &str {
        "elicit_select"
    }

    fn description(&self) -> &str {
        "Select one option from a finite list of choices"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to display"
                },
                "options": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Array of valid options to choose from"
                }
            },
            "required": ["prompt", "options"]
        })
    }

    #[instrument(skip(self, input), fields(tool = "elicit_select"))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'prompt' parameter"))?;

        let options: Vec<String> = input
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .ok_or_else(|| McpError::invalid_input("Missing or invalid 'options' parameter"))?;

        if options.is_empty() {
            return Err(McpError::invalid_input("Options array cannot be empty"));
        }

        // Convert to &str array for dialog API
        let option_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();

        let index = self
            .dialog
            .ask_choice(prompt, &option_refs)
            .await
            .map_err(|e| McpError::execution_failed(format!("Dialog error: {}", e)))?;

        let selected = options.get(index).ok_or_else(|| {
            McpError::execution_failed(format!("Invalid index {} (max {})", index, options.len()))
        })?;

        // Return the selected value directly (not wrapped in object)
        // The elicitation crate expects: "OptionA" not {"value": "OptionA"}
        Ok(json!(selected))
    }
}

