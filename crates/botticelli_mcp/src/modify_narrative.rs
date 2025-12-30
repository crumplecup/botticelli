//! Modify narrative tool types for updating existing narratives.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for modifying an existing narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyNarrativeParams {
    /// Existing narrative TOML to modify
    pub narrative_toml: String,

    /// Natural language description of the modification
    pub modification: String,

    /// Optional path to save modified narrative
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_to: Option<String>,
}

/// Result from modifying a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ModifyNarrativeResult {
    /// Modified narrative TOML
    pub toml: String,

    /// Validation results
    pub validation: Value,

    /// List of changes applied
    pub changes: Vec<String>,

    /// Path where narrative was saved (if save_to was provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_to: Option<String>,
}

impl ModifyNarrativeResult {
    /// Create a new modification result.
    pub fn new(toml: String, validation: Value, changes: Vec<String>, saved_to: Option<String>) -> Self {
        Self {
            toml,
            validation,
            changes,
            saved_to,
        }
    }
}
