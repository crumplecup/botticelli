//! Modify narrative tool types for updating existing narratives.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for modifying an existing narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct ModifyNarrativeParams {
    /// Existing narrative TOML to modify
    narrative_toml: String,

    /// Natural language description of the modification
    modification: String,

    /// Optional path to save modified narrative
    #[serde(skip_serializing_if = "Option::is_none")]
    save_to: Option<String>,
}

impl ModifyNarrativeParams {
    /// Create new modify narrative parameters.
    #[tracing::instrument(skip(narrative_toml, modification), fields(narrative_len = narrative_toml.len(), modification_len = modification.len()))]
    pub fn new(narrative_toml: String, modification: String, save_to: Option<String>) -> Self {
        Self {
            narrative_toml,
            modification,
            save_to,
        }
    }
}

/// Result from modifying a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ModifyNarrativeResult {
    /// Modified narrative TOML
    toml: String,

    /// Validation results
    validation: Value,

    /// List of changes applied
    changes: Vec<String>,

    /// Path where narrative was saved (if save_to was provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    saved_to: Option<String>,
}

impl ModifyNarrativeResult {
    /// Create a new modification result.
    #[tracing::instrument(skip(toml, validation, changes), fields(toml_len = toml.len(), changes_count = changes.len()))]
    pub fn new(
        toml: String,
        validation: Value,
        changes: Vec<String>,
        saved_to: Option<String>,
    ) -> Self {
        Self {
            toml,
            validation,
            changes,
            saved_to,
        }
    }
}
