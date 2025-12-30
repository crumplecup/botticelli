//! Save narrative tool types for persisting narratives to files.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for saving a narrative to a file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SaveNarrativeParams {
    /// Narrative TOML content to save
    pub narrative_toml: String,

    /// Path where to save the file (should end in .toml)
    pub file_path: String,

    /// Allow overwriting existing file
    #[serde(default)]
    pub overwrite: bool,
}

/// Result from saving a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SaveNarrativeResult {
    /// Status of the save operation
    pub status: String,

    /// Absolute path where file was saved
    pub file_path: String,

    /// Size of the saved file in bytes
    pub size_bytes: usize,

    /// Whether an existing file was overwritten
    pub overwritten: bool,
}

impl SaveNarrativeResult {
    /// Create a new save result.
    pub fn new(file_path: String, size_bytes: usize, overwritten: bool) -> Self {
        Self {
            status: "saved".to_string(),
            file_path,
            size_bytes,
            overwritten,
        }
    }
}
