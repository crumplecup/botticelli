//! Create narrative tool types for generating narratives from descriptions.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for creating a narrative from natural language.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateNarrativeParams {
    /// Natural language description of the narrative workflow
    pub description: String,

    /// Narrative name (alphanumeric + underscores)
    pub name: String,

    /// Optional default model for all acts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,

    /// Optional default temperature (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_temperature: Option<f64>,
}

/// Result from creating a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_new::new, derive_getters::Getters)]
pub struct CreateNarrativeResult {
    /// Generated narrative TOML
    toml: String,

    /// TOML with helpful comments added
    toml_with_comments: String,

    /// Validation results
    validation: Value,

    /// Human-readable summary
    summary: String,

    /// List of auto-fixes that were applied
    auto_fixes_applied: Vec<String>,

    /// Number of acts in the narrative
    act_count: usize,
}
