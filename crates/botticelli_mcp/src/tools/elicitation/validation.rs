use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Input for validating a narrative
pub struct ValidateNarrativeInput {
    /// Narrative ID to validate
    pub narrative_id: String,
    /// Enable strict validation mode
    #[serde(default)]
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Output from narrative validation
pub struct ValidateNarrativeOutput {
    /// Whether the narrative is valid
    pub valid: bool,
    /// Validation errors found
    pub errors: Vec<ValidationIssue>,
    /// Validation warnings
    pub warnings: Vec<ValidationIssue>,
    /// Completeness analysis
    pub completeness: CompletenessReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub field: String,
    pub message: String,
    pub suggestion: String,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletenessReport {
    pub metadata: String,
    pub acts: String,
    pub inputs: String,
    pub overall: String,
}

/// Input for applying automatic validation fixes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyValidationFixesInput {
    /// Narrative ID to fix
    pub narrative_id: String,
    /// Types of fixes to apply
    pub fix_types: Vec<String>,
    /// Confirm before applying fixes
    #[serde(default)]
    pub confirm: bool,
}

/// Output from applying validation fixes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyValidationFixesOutput {
    /// Whether fixes were successfully applied
    pub success: bool,
    /// List of fixes that were applied
    pub fixes_applied: Vec<String>,
    /// Number of errors remaining after fixes
    pub remaining_errors: usize,
}

// Legacy helper functions - unused, replaced by rmcp handlers
// Kept for backward compatibility with external crates
