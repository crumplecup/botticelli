use serde::{Deserialize, Serialize};

/// Input for getting narrative state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNarrativeStateInput {
    /// Narrative UUID
    pub narrative_id: String,
    /// Output format (summary, full, or toml)
    #[serde(default = "default_format")]
    pub format: StateFormat,
}

fn default_format() -> StateFormat {
    StateFormat::Summary
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StateFormat {
    Summary,
    Full,
    Toml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Output containing narrative state
pub struct GetNarrativeStateOutput {
    /// Narrative ID
    pub narrative_id: String,
    /// State summary
    pub state: NarrativeStateSummary,
    /// Optional TOML representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toml: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeStateSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub acts_count: usize,
    pub acts: Vec<String>,
    pub completeness: String,
    pub has_carousel: bool,
}

// Legacy helper function - unused, replaced by rmcp handler
// Kept to maintain backward compatibility for external crates
// that may reference these types/functions
