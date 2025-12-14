//! MCP tools for session-based narrative elicitation.

use crate::tools::elicitation::registry::NarrativeRegistry;
use crate::tools::McpTool;
use crate::NarrativeHelper;
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use serde_json::{json, Value};
use tracing::{debug, instrument};
use uuid::Uuid;

/// Tool for creating a new narrative elicitation session.
pub struct CreateNarrativeSessionTool {
    registry: NarrativeRegistry,
}

impl CreateNarrativeSessionTool {
    /// Create tool with registry.
    pub fn new(registry: NarrativeRegistry) -> Self {
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
        let state = json!({
            "description": description,
            "suggested_name": suggested_name,
            "acts": acts.iter().map(|a| json!({
                "name": a.name,
                "prompt": a.prompt
            })).collect::<Vec<_>>(),
            "metadata": {}
        });

        let narrative_id = self.registry.create_session(state);

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
    registry: NarrativeRegistry,
}

impl ElicitMetadataTool {
    /// Create tool with registry.
    pub fn new(registry: NarrativeRegistry) -> Self {
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

        let mut state = self
            .registry
            .get(&narrative_id)
            .ok_or_else(|| McpError::invalid_input("Narrative session not found".to_string()))?;

        // Update metadata fields if provided
        let metadata = state["metadata"].as_object().cloned().unwrap_or_default();
        let mut metadata = metadata;

        if let Some(name) = input.get("name").and_then(|v| v.as_str()) {
            metadata.insert("name".to_string(), json!(name));
        }

        if let Some(desc) = input.get("description").and_then(|v| v.as_str()) {
            metadata.insert("description".to_string(), json!(desc));
        }

        if let Some(model) = input.get("default_model").and_then(|v| v.as_str()) {
            metadata.insert("default_model".to_string(), json!(model));
        }

        if let Some(temp) = input.get("default_temperature").and_then(|v| v.as_f64()) {
            metadata.insert("default_temperature".to_string(), json!(temp));
        }

        state["metadata"] = json!(metadata);
        self.registry.update(&narrative_id, state.clone());

        debug!(narrative_id = %narrative_id, "Metadata updated");

        Ok(json!({
            "narrative_id": narrative_id.to_string(),
            "metadata": metadata,
            "status": "updated"
        }))
    }
}

/// Tool for adding or updating an act.
pub struct ElicitActTool {
    registry: NarrativeRegistry,
}

impl ElicitActTool {
    /// Create tool with registry.
    pub fn new(registry: NarrativeRegistry) -> Self {
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

        let mut state = self
            .registry
            .get(&narrative_id)
            .ok_or_else(|| McpError::invalid_input("Narrative session not found".to_string()))?;

        // Update or add act
        let mut acts = state["acts"].as_array().cloned().unwrap_or_default();

        let mut act = json!({
            "name": act_name,
            "prompt": prompt
        });

        if let Some(model) = input.get("model").and_then(|v| v.as_str()) {
            act["model"] = json!(model);
        }

        if let Some(temp) = input.get("temperature").and_then(|v| v.as_f64()) {
            act["temperature"] = json!(temp);
        }

        // Replace if exists, otherwise add
        if let Some(pos) = acts.iter().position(|a| a["name"] == act_name) {
            acts[pos] = act;
        } else {
            acts.push(act);
        }

        state["acts"] = json!(acts);
        self.registry.update(&narrative_id, state.clone());

        debug!(narrative_id = %narrative_id, act_name, "Act updated");

        Ok(json!({
            "narrative_id": narrative_id.to_string(),
            "act_name": act_name,
            "status": "updated",
            "total_acts": acts.len()
        }))
    }
}

/// Tool for finalizing and generating TOML.
pub struct FinalizeNarrativeTool {
    registry: NarrativeRegistry,
}

impl FinalizeNarrativeTool {
    /// Create tool with registry.
    pub fn new(registry: NarrativeRegistry) -> Self {
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

        // Generate TOML from state
        let toml = generate_toml_from_state(&state)?;

        debug!(narrative_id = %narrative_id, "Narrative finalized");

        Ok(json!({
            "narrative_id": narrative_id.to_string(),
            "toml": toml,
            "status": "finalized"
        }))
    }
}

/// Generate TOML from session state.
fn generate_toml_from_state(state: &Value) -> McpResult<String> {
    let metadata = &state["metadata"];
    let acts = state["acts"]
        .as_array()
        .ok_or_else(|| McpError::invalid_input("Missing acts".to_string()))?;

    let name = metadata["name"]
        .as_str()
        .ok_or_else(|| McpError::invalid_input("Missing name".to_string()))?;

    let description = metadata["description"]
        .as_str()
        .unwrap_or("Generated narrative");

    let mut toml = String::new();

    // [narrative] section
    toml.push_str("[narrative]\n");
    toml.push_str(&format!("name = \"{}\"\n", name));
    toml.push_str(&format!("description = \"{}\"\n", description));

    if let Some(model) = metadata["default_model"].as_str() {
        toml.push_str(&format!("model = \"{}\"\n", model));
    }

    if let Some(temp) = metadata["default_temperature"].as_f64() {
        toml.push_str(&format!("temperature = {}\n", temp));
    }

    toml.push('\n');

    // [toc] section
    toml.push_str("[toc]\n");
    toml.push_str("order = [");
    for (i, act) in acts.iter().enumerate() {
        if i > 0 {
            toml.push_str(", ");
        }
        let act_name = act["name"].as_str().unwrap_or("unknown");
        toml.push_str(&format!("\"{}\"", act_name));
    }
    toml.push_str("]\n\n");

    // [acts] section
    toml.push_str("[acts]\n");
    for act in acts {
        let act_name = act["name"].as_str().unwrap_or("unknown");
        let prompt = act["prompt"].as_str().unwrap_or("");
        toml.push_str(&format!(
            "{} = \"{}\"\n",
            act_name,
            NarrativeHelper::escape_toml_string(prompt)
        ));
    }

    Ok(toml)
}
