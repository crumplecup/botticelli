/// MCP tool for eliciting narrative metadata.

use crate::tools::McpTool;
use botticelli_error::{McpError, McpResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, info, instrument};
use uuid::Uuid;

use super::{helpers, NarrativeRegistry};

/// Tool for setting or updating narrative metadata.
pub struct ElicitMetadataTool {
    registry: Arc<NarrativeRegistry>,
}

impl ElicitMetadataTool {
    pub fn new(registry: Arc<NarrativeRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl McpTool for ElicitMetadataTool {
    fn name(&self) -> &str {
        "elicit_metadata"
    }

    fn description(&self) -> &str {
        "Set or update narrative metadata (name, description, model defaults). \
         Use after creating a session or when user wants to modify metadata. \
         Partial updates are supported - only provided fields will be updated."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {
                    "type": "string",
                    "description": "UUID of the narrative session"
                },
                "name": {
                    "type": "string",
                    "description": "Optional - narrative name (alphanumeric, _, -)"
                },
                "description": {
                    "type": "string",
                    "description": "Optional - narrative description"
                },
                "model": {
                    "type": "string",
                    "description": "Optional - default model (e.g., claude-3-5-sonnet-20241022)"
                },
                "temperature": {
                    "type": "number",
                    "description": "Optional - default temperature (0.0-2.0)"
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Optional - default max tokens"
                }
            },
            "required": ["narrative_id"]
        })
    }

    #[instrument(skip(self, input))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        debug!("Eliciting metadata");

        let narrative_id_str = input
            .get("narrative_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'narrative_id'".to_string()))?;

        let narrative_id = Uuid::parse_str(narrative_id_str)
            .map_err(|e| McpError::invalid_input(format!("Invalid UUID: {}", e)))?;

        let mut state = self
            .registry
            .get(&narrative_id)
            .ok_or_else(|| McpError::invalid_input(format!("Narrative {} not found", narrative_id)))?;

        let mut validation_warnings = Vec::new();
        let mut missing_required = Vec::new();

        // Update name if provided
        if let Some(name) = input.get("name").and_then(|v| v.as_str()) {
            if !helpers::is_valid_name(name) {
                return Err(McpError::invalid_input(format!(
                    "Invalid name '{}': must be alphanumeric with _, - only, max 64 chars",
                    name
                )));
            }
            state["name"] = json!(name);
        }

        // Update description if provided
        if let Some(desc) = input.get("description").and_then(|v| v.as_str()) {
            state["description"] = json!(desc);
        }

        // Update model if provided
        if let Some(model) = input.get("model").and_then(|v| v.as_str()) {
            state["model"] = json!(model);
        }

        // Update temperature if provided
        if let Some(temp) = input.get("temperature").and_then(|v| v.as_f64()) {
            if !(0.0..=2.0).contains(&temp) {
                validation_warnings.push("Temperature outside recommended range 0.0-2.0");
            }
            state["temperature"] = json!(temp);
        }

        // Update max_tokens if provided
        if let Some(max_tokens) = input.get("max_tokens").and_then(|v| v.as_i64()) {
            if max_tokens <= 0 {
                return Err(McpError::invalid_input("max_tokens must be positive".to_string()));
            }
            state["max_tokens"] = json!(max_tokens);
        }

        // Check completeness
        if state.get("name").is_none() {
            missing_required.push("name");
        }
        if state.get("description").is_none() {
            missing_required.push("description");
        }

        // Update registry
        self.registry.update(&narrative_id, state.clone());

        info!(narrative_id = %narrative_id, "Updated metadata");

        Ok(json!({
            "success": true,
            "current_metadata": {
                "name": state.get("name"),
                "description": state.get("description"),
                "model": state.get("model"),
                "temperature": state.get("temperature"),
                "max_tokens": state.get("max_tokens")
            },
            "missing_required": missing_required,
            "validation_warnings": validation_warnings
        }))
    }
}
