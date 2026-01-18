//! Save narrative tool types for persisting narratives to files.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for saving a narrative to a file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct SaveNarrativeParams {
    /// Narrative TOML content to save
    narrative_toml: String,

    /// Path where to save the file (should end in .toml)
    file_path: String,

    /// Allow overwriting existing file
    #[serde(default)]
    overwrite: bool,
}

impl SaveNarrativeParams {
    /// Create new save narrative parameters.
    #[tracing::instrument(skip(narrative_toml, file_path), fields(toml_len = narrative_toml.len(), file_path = %file_path))]
    pub fn new(narrative_toml: String, file_path: String, overwrite: bool) -> Self {
        Self {
            narrative_toml,
            file_path,
            overwrite,
        }
    }
}

/// Result from saving a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct SaveNarrativeResult {
    /// Status of the save operation
    status: String,

    /// Absolute path where file was saved
    file_path: String,

    /// Size of the saved file in bytes
    size_bytes: usize,

    /// Whether an existing file was overwritten
    overwritten: bool,
}

impl SaveNarrativeResult {
    /// Create a new save result.
    #[tracing::instrument(skip(file_path), fields(file_path = %file_path))]
    pub fn new(file_path: String, size_bytes: usize, overwritten: bool) -> Self {
        Self {
            status: "saved".to_string(),
            file_path,
            size_bytes,
            overwritten,
        }
    }
}
