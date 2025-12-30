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
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CreateNarrativeResult {
    /// Generated narrative TOML
    pub toml: String,

    /// TOML with helpful comments added
    pub toml_with_comments: String,

    /// Validation results
    pub validation: Value,

    /// Human-readable summary
    pub summary: String,

    /// List of auto-fixes that were applied
    pub auto_fixes_applied: Vec<String>,

    /// Number of acts in the narrative
    pub act_count: usize,
}

impl CreateNarrativeResult {
    /// Create a new narrative creation result.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        toml: String,
        toml_with_comments: String,
        validation: Value,
        summary: String,
        auto_fixes_applied: Vec<String>,
        act_count: usize,
    ) -> Self {
        Self {
            toml,
            toml_with_comments,
            validation,
            summary,
            auto_fixes_applied,
            act_count,
        }
    }
}
