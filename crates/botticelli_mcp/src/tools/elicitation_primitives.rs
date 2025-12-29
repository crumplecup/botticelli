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

/// Tool for eliciting numeric input with range constraints.
///
/// This tool corresponds to the `elicit_number` MCP tool expected by
/// the elicitation crate for integer and numeric types.
pub struct ElicitNumberTool {
    dialog: Arc<DialogResource>,
}

impl ElicitNumberTool {
    /// Create a new number elicitation tool.
    pub fn new(dialog: Arc<DialogResource>) -> Self {
        Self { dialog }
    }
}

#[async_trait]
impl McpTool for ElicitNumberTool {
    fn name(&self) -> &str {
        "elicit_number"
    }

    fn description(&self) -> &str {
        "Elicit a number within a specified range (min and max inclusive)"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to display"
                },
                "min": {
                    "type": "integer",
                    "description": "Minimum acceptable value (inclusive)"
                },
                "max": {
                    "type": "integer",
                    "description": "Maximum acceptable value (inclusive)"
                }
            },
            "required": ["prompt", "min", "max"]
        })
    }

    #[instrument(skip(self, input), fields(tool = "elicit_number"))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'prompt' parameter"))?;

        let min = input
            .get("min")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| McpError::invalid_input("Missing 'min' parameter"))?;

        let max = input
            .get("max")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| McpError::invalid_input("Missing 'max' parameter"))?;

        if min > max {
            return Err(McpError::invalid_input(format!(
                "Invalid range: min ({}) > max ({})",
                min, max
            )));
        }

        let num = self
            .dialog
            .ask_number(prompt, min, max)
            .await
            .map_err(|e| McpError::execution_failed(format!("Dialog error: {}", e)))?;

        // Return the number value directly (not wrapped in object)
        // The elicitation crate expects: 42 not {"value": 42}
        Ok(json!(num))
    }
}

