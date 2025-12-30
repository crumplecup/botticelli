//! Validate narrative tool types.
//!
//! Validates narrative TOML files for syntax, structure, and references.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for validating a narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateNarrativeParams {
    /// TOML content to validate (either this or file_path must be provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    
    /// Path to TOML file to validate (either this or content must be provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    
    /// Check that media and nested narrative files exist.
    #[serde(default)]
    pub validate_files: bool,
    
    /// Warn on unknown model names.
    #[serde(default = "default_validate_models")]
    pub validate_models: bool,
    
    /// Warn about unused resources (bots, tables, media).
    #[serde(default = "default_warn_unused")]
    pub warn_unused: bool,
    
    /// Treat warnings as errors.
    #[serde(default)]
    pub strict: bool,
}

fn default_validate_models() -> bool {
    true
}

fn default_warn_unused() -> bool {
    true
}

/// Location information for a validation issue.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValidationLocation {
    /// Line number in the TOML file.
    pub line: usize,
    
    /// Column number in the TOML file.
    pub column: usize,
    
    /// Section where the issue occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

/// A validation error with location and suggestion.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValidationError {
    /// Kind of error (e.g., "SyntaxError", "ReferenceError").
    pub kind: String,
    
    /// Error message.
    pub message: String,
    
    /// Optional suggestion for fixing the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    
    /// Optional location information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<ValidationLocation>,
}

/// A validation warning with location.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValidationWarning {
    /// Kind of warning (e.g., "UnknownModel", "UnusedResource").
    pub kind: String,
    
    /// Warning message.
    pub message: String,
    
    /// Optional location information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<ValidationLocation>,
}

/// Result from validating a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValidateNarrativeResult {
    /// Whether the narrative is valid (no errors, or no warnings if strict).
    pub valid: bool,
    
    /// List of validation errors.
    pub errors: Vec<ValidationError>,
    
    /// List of validation warnings.
    pub warnings: Vec<ValidationWarning>,
    
    /// Summary message.
    pub summary: String,
}

impl ValidateNarrativeResult {
    /// Create a new validation result.
    pub fn new(
        valid: bool,
        errors: Vec<ValidationError>,
        warnings: Vec<ValidationWarning>,
    ) -> Self {
        let summary = format!("{} error(s), {} warning(s)", errors.len(), warnings.len());
        Self {
            valid,
            errors,
            warnings,
            summary,
        }
    }
}
