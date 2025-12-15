//! Elicitation tool implementations for conversational narrative creation.

use crate::{McpClientError, McpClientErrorKind, McpClientResult, ToolHandler};
use async_trait::async_trait;
use pmcp::{Content, ToolInfo};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Registry for managing active narrative elicitation sessions.
#[derive(Debug, Clone)]
pub struct ElicitationRegistry {
    sessions: Arc<RwLock<HashMap<Uuid, Value>>>,
}

impl ElicitationRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new elicitation session.
    pub fn create_session(&self, state: Value) -> Uuid {
        let id = Uuid::new_v4();
        let mut sessions = self.sessions.write().expect("Registry lock poisoned");
        sessions.insert(id, state);
        tracing::info!(session_id = %id, "Created elicitation session");
        id
    }

    /// Get session state by UUID.
    pub fn get(&self, id: &Uuid) -> Option<Value> {
        let sessions = self.sessions.read().expect("Registry lock poisoned");
        sessions.get(id).cloned()
    }

    /// Update session state.
    pub fn update(&self, id: &Uuid, state: Value) -> bool {
        let mut sessions = self.sessions.write().expect("Registry lock poisoned");
        if sessions.contains_key(id) {
            sessions.insert(*id, state);
            true
        } else {
            false
        }
    }

    /// Remove and return session state.
    pub fn remove(&self, id: &Uuid) -> Option<Value> {
        let mut sessions = self.sessions.write().expect("Registry lock poisoned");
        sessions.remove(id)
    }
}

impl Default for ElicitationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool for creating a new narrative elicitation session.
#[derive(Debug, Clone)]
pub struct CreateElicitationSessionTool {
    registry: ElicitationRegistry,
}

impl CreateElicitationSessionTool {
    /// Create tool with registry.
    pub fn new(registry: ElicitationRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ToolHandler for CreateElicitationSessionTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "create_elicitation_session",
            Some(
                "Initialize a new conversational narrative elicitation session. \
                 Analyzes user description and returns session ID for tracking."
                    .to_string(),
            ),
            json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "User's description of what they want to create"
                    }
                },
                "required": ["description"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpClientResult<Vec<Content>> {
        let description = input
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing 'description'".to_string(),
                ))
            })?;

        tracing::debug!(description, "Creating elicitation session");

        // Analyze description
        let acts = extract_acts_from_description(description);
        let suggested_name = suggest_name_from_description(description);

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
            "acts": acts,
            "metadata": {}
        });

        let session_id = self.registry.create_session(state);

        let result = json!({
            "session_id": session_id.to_string(),
            "suggested_name": suggested_name,
            "analysis": {
                "detected_acts": acts.iter().map(|a| a["name"].as_str().unwrap_or("unknown")).collect::<Vec<_>>(),
                "complexity": complexity,
                "act_count": acts.len()
            }
        });

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        }])
    }
}

/// Tool for setting narrative metadata during elicitation.
#[derive(Debug, Clone)]
pub struct ElicitMetadataTool {
    registry: ElicitationRegistry,
}

impl ElicitMetadataTool {
    /// Create tool with registry.
    pub fn new(registry: ElicitationRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ToolHandler for ElicitMetadataTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "elicit_metadata",
            Some("Set or update narrative metadata (name, description, defaults)".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Elicitation session UUID"
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
                "required": ["session_id"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpClientResult<Vec<Content>> {
        let session_id = input
            .get("session_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Invalid session_id".to_string(),
                ))
            })?;

        let mut state = self.registry.get(&session_id).ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Session not found".to_string(),
            ))
        })?;

        // Update metadata fields if provided
        let mut metadata = state["metadata"].as_object().cloned().unwrap_or_default();

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
        self.registry.update(&session_id, state.clone());

        let result = json!({
            "session_id": session_id.to_string(),
            "metadata": metadata,
            "status": "updated"
        });

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        }])
    }
}

/// Tool for adding or updating acts during elicitation.
#[derive(Debug, Clone)]
pub struct ElicitActTool {
    registry: ElicitationRegistry,
}

impl ElicitActTool {
    /// Create tool with registry.
    pub fn new(registry: ElicitationRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ToolHandler for ElicitActTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "elicit_act",
            Some("Add or update an act in the narrative being elicited".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Elicitation session UUID"
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
                        "description": "Model override for this act"
                    },
                    "temperature": {
                        "type": "number",
                        "description": "Temperature override for this act"
                    }
                },
                "required": ["session_id", "act_name", "prompt"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpClientResult<Vec<Content>> {
        let session_id = input
            .get("session_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Invalid session_id".to_string(),
                ))
            })?;

        let act_name = input
            .get("act_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing 'act_name'".to_string(),
                ))
            })?;

        let prompt = input.get("prompt").and_then(|v| v.as_str()).ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Missing 'prompt'".to_string(),
            ))
        })?;

        let mut state = self.registry.get(&session_id).ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Session not found".to_string(),
            ))
        })?;

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
        self.registry.update(&session_id, state.clone());

        let result = json!({
            "session_id": session_id.to_string(),
            "act_name": act_name,
            "status": "updated",
            "total_acts": acts.len()
        });

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        }])
    }
}

/// Tool for finalizing elicitation and generating TOML.
#[derive(Debug, Clone)]
pub struct FinalizeElicitationTool {
    registry: ElicitationRegistry,
}

impl FinalizeElicitationTool {
    /// Create tool with registry.
    pub fn new(registry: ElicitationRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ToolHandler for FinalizeElicitationTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "finalize_elicitation",
            Some("Complete the elicitation session and generate narrative TOML file".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Elicitation session UUID"
                    }
                },
                "required": ["session_id"]
            }),
        )
    }

    #[tracing::instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpClientResult<Vec<Content>> {
        let session_id = input
            .get("session_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Invalid session_id".to_string(),
                ))
            })?;

        let state = self.registry.remove(&session_id).ok_or_else(|| {
            McpClientError::new(McpClientErrorKind::InvalidToolCall(
                "Session not found".to_string(),
            ))
        })?;

        // Generate TOML from state
        let toml = generate_toml_from_state(&state)?;

        let result = json!({
            "session_id": session_id.to_string(),
            "toml": toml,
            "status": "finalized"
        });

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        }])
    }
}

// Helper functions

fn extract_acts_from_description(description: &str) -> Vec<Value> {
    // Simple heuristic: look for numbered sections or "then"/"next" patterns
    let description_lower = description.to_lowercase();

    if description_lower.contains("then") || description_lower.contains("next") {
        // Multi-act narrative
        vec![
            json!({"name": "setup", "prompt": "Set up the scenario"}),
            json!({"name": "development", "prompt": "Develop the main content"}),
            json!({"name": "conclusion", "prompt": "Conclude the narrative"}),
        ]
    } else {
        // Single act
        vec![json!({"name": "main", "prompt": description})]
    }
}

fn suggest_name_from_description(description: &str) -> String {
    // Extract first few words as suggestion
    let words: Vec<&str> = description.split_whitespace().take(3).collect();
    words
        .join("_")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn generate_toml_from_state(state: &Value) -> McpClientResult<String> {
    let metadata = &state["metadata"];
    let acts = state["acts"].as_array().ok_or_else(|| {
        McpClientError::new(McpClientErrorKind::InvalidToolCall("Missing acts".to_string()))
    })?;

    let name = metadata["name"].as_str().ok_or_else(|| {
        McpClientError::new(McpClientErrorKind::InvalidToolCall("Missing name".to_string()))
    })?;

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
        toml.push_str(&format!("{} = \"{}\"\n", act_name, escape_toml_string(prompt)));
    }

    Ok(toml)
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
