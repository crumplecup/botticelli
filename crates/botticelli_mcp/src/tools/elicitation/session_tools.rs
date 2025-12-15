//! MCP tools for session-based narrative elicitation.

use crate::elicitation::{PartialAct, PartialNarrative};
use crate::tools::elicitation::PartialNarrativeRegistry;
use crate::tools::McpTool;
use crate::NarrativeHelper;
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use serde_json::{json, Value};
use tracing::{debug, instrument};
use uuid::Uuid;

/// Tool for creating a new narrative elicitation session.
pub struct CreateNarrativeSessionTool {
    registry: PartialNarrativeRegistry,
}

impl CreateNarrativeSessionTool {
    /// Create tool with registry.
    pub fn new(registry: PartialNarrativeRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl McpTool for CreateNarrativeSessionTool {
    fn name(&self) -> &str {
        "create_narrative_session"
    }

    fn description(&self) -> &str {
        "Initialize a new narrative creation session. Returns a session ID and \
         analysis of the user's description to guide next steps."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "User's description of what they want to create"
                }
            },
            "required": ["description"]
        })
    }

    #[instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let description = input
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'description'".to_string()))?;

        debug!(description, "Creating narrative session");

        // Analyze description
        let acts = NarrativeHelper::extract_acts_from_description(description);
        let suggested_name = NarrativeHelper::suggest_name_from_description(description);

        let complexity = if acts.len() == 1 {
            "simple"
        } else if acts.len() <= 3 {
            "moderate"
        } else {
            "complex"
        };

        // Initialize session state
        let mut partial = PartialNarrative::new();
        partial.description = Some(description.to_string());
        partial.name = Some(suggested_name.clone());

        // Add acts
        for act in &acts {
            partial.acts.insert(
                act.name.clone(),
                PartialAct::new(act.prompt.clone(), None, None, vec![], None),
            );
            partial.act_order.push(act.name.clone());
        }

        let narrative_id = self.registry.create_session(partial);

        debug!(narrative_id = %narrative_id, acts = acts.len(), "Session created");

        Ok(json!({
            "narrative_id": narrative_id.to_string(),
            "suggested_name": suggested_name,
            "analysis": {
                "detected_acts": acts.iter().map(|a| &a.name).collect::<Vec<_>>(),
                "complexity": complexity,
                "act_count": acts.len()
            }
        }))
    }
}

/// Tool for setting narrative metadata.
pub struct ElicitMetadataTool {
    registry: PartialNarrativeRegistry,
}

impl ElicitMetadataTool {
    /// Create tool with registry.
    pub fn new(registry: PartialNarrativeRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl McpTool for ElicitMetadataTool {
    fn name(&self) -> &str {
        "elicit_metadata"
    }

    fn description(&self) -> &str {
        "Set or update narrative metadata (name, description, defaults)"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {
                    "type": "string",
                    "description": "Session UUID"
                },
                "name": {
                    "type": "string",
                    "description": "Narrative name"
                },
                "description": {
                    "type": "string",
                    "description": "Narrative description"
                },
                "default_model": {
                    "type": "string",
                    "description": "Default model for acts"
                },
                "default_temperature": {
                    "type": "number",
                    "description": "Default temperature (0.0-1.0)"
                }
            },
            "required": ["narrative_id"]
        })
    }

    #[instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let narrative_id = input
            .get("narrative_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| McpError::invalid_input("Invalid narrative_id".to_string()))?;

        // Update fields if provided
        self.registry
            .update_narrative(narrative_id, |partial| {
                if let Some(name) = input.get("name").and_then(|v| v.as_str()) {
                    partial.name = Some(name.to_string());
                }

                if let Some(desc) = input.get("description").and_then(|v| v.as_str()) {
                    partial.description = Some(desc.to_string());
                }

                if let Some(model) = input.get("default_model").and_then(|v| v.as_str()) {
                    partial.model = Some(model.to_string());
                }

                if let Some(temp) = input.get("default_temperature").and_then(|v| v.as_f64()) {
                    partial.temperature = Some(temp);
                }

                Ok(())
            })
            .await?;

        debug!(narrative_id = %narrative_id, "Metadata updated");

        Ok(json!({
            "narrative_id": narrative_id.to_string(),
            "status": "updated"
        }))
    }
}

/// Tool for adding or updating an act.
pub struct ElicitActTool {
    registry: PartialNarrativeRegistry,
}

impl ElicitActTool {
    /// Create tool with registry.
    pub fn new(registry: PartialNarrativeRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl McpTool for ElicitActTool {
    fn name(&self) -> &str {
        "elicit_act"
    }

    fn description(&self) -> &str {
        "Add or update an act in the narrative"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {
                    "type": "string",
                    "description": "Session UUID"
                },
                "act_name": {
                    "type": "string",
                    "description": "Act identifier"
                },
                "prompt": {
                    "type": "string",
                    "description": "Act prompt"
                },
                "model": {
                    "type": "string",
                    "description": "Model override"
                },
                "temperature": {
                    "type": "number",
                    "description": "Temperature override"
                }
            },
            "required": ["narrative_id", "act_name", "prompt"]
        })
    }

    #[instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let narrative_id = input
            .get("narrative_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| McpError::invalid_input("Invalid narrative_id".to_string()))?;

        let act_name = input
            .get("act_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'act_name'".to_string()))?;

        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'prompt'".to_string()))?;

        let mut state = self.registry.get_narrative(narrative_id)?;

        // Update or add act
        let model = input
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from);
        let temperature = input.get("temperature").and_then(|v| v.as_f64());

        let act = PartialAct::new(prompt.to_string(), model, temperature, vec![], None);

        // Add to registry
        self.registry
            .update_narrative(narrative_id, |partial| {
                partial.acts.insert(act_name.to_string(), act.clone());
                if !partial.act_order.contains(&act_name.to_string()) {
                    partial.act_order.push(act_name.to_string());
                }
                Ok(())
            })
            .await?;

        // Get updated count
        let partial = self.registry.get_narrative(narrative_id)?;
        let acts_count = partial.acts.len();

        debug!(narrative_id = %narrative_id, act_name, "Act updated");

        Ok(json!({
            "narrative_id": narrative_id.to_string(),
            "act_name": act_name,
            "status": "updated",
            "total_acts": acts_count
        }))
    }
}

/// Tool for finalizing and generating TOML.
pub struct FinalizeNarrativeTool {
    registry: PartialNarrativeRegistry,
}

impl FinalizeNarrativeTool {
    /// Create tool with registry.
    pub fn new(registry: PartialNarrativeRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl McpTool for FinalizeNarrativeTool {
    fn name(&self) -> &str {
        "finalize_narrative"
    }

    fn description(&self) -> &str {
        "Complete the narrative session and generate final TOML"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {
                    "type": "string",
                    "description": "Session UUID"
                }
            },
            "required": ["narrative_id"]
        })
    }

    #[instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let narrative_id = input
            .get("narrative_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| McpError::invalid_input("Invalid narrative_id".to_string()))?;

        let state = self
            .registry
            .remove(&narrative_id)
            .ok_or_else(|| McpError::invalid_input("Narrative session not found".to_string()))?;

        // Generate TOML from PartialNarrative
        let toml = state.to_toml()?;

        debug!(narrative_id = %narrative_id, "Narrative finalized");

        Ok(json!({
            "narrative_id": narrative_id.to_string(),
            "toml": toml,
            "status": "finalized"
        }))
    }
}
