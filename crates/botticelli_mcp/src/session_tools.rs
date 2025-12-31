//! Session-based narrative elicitation tool types.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for creating a narrative session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for initializing a new narrative creation session")]
pub struct CreateNarrativeSessionParams {
    /// User's description of what they want to create.
    #[schemars(description = "User's description of the narrative they want to create")]
    pub description: String,
}

/// Detected act information from description analysis.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Information about a detected act in the narrative")]
pub struct DetectedAct {
    /// Act name.
    #[schemars(description = "Name of the detected act")]
    pub name: String,
}

/// Analysis of the user's description.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Analysis of the user's narrative description")]
pub struct NarrativeAnalysis {
    /// Names of detected acts.
    #[schemars(description = "List of detected act names")]
    pub detected_acts: Vec<String>,

    /// Complexity assessment.
    #[schemars(description = "Complexity assessment: simple, moderate, or complex")]
    pub complexity: String,

    /// Number of acts detected.
    #[schemars(description = "Number of acts detected in the description")]
    pub act_count: usize,
}

/// Result from creating a narrative session.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result from creating a new narrative session")]
pub struct CreateNarrativeSessionResult {
    /// UUID of the created session.
    #[schemars(description = "UUID of the created narrative session")]
    pub narrative_id: String,

    /// Suggested name for the narrative.
    #[schemars(description = "Suggested name based on the description")]
    pub suggested_name: String,

    /// Analysis of the description.
    #[schemars(description = "Analysis of the user's description")]
    pub analysis: NarrativeAnalysis,
}

/// Parameters for setting narrative metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for setting or updating narrative metadata")]
pub struct ElicitMetadataParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    pub narrative_id: String,

    /// Narrative name.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Name of the narrative")]
    pub name: Option<String>,

    /// Narrative description.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Description of the narrative")]
    pub description: Option<String>,

    /// Default model for acts.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Default model to use for acts")]
    pub default_model: Option<String>,

    /// Default temperature (0.0-1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Default temperature (0.0-1.0)")]
    pub default_temperature: Option<f64>,
}

/// Result from eliciting metadata.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result from updating narrative metadata")]
pub struct ElicitMetadataResult {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    pub narrative_id: String,

    /// Status message.
    #[schemars(description = "Status message (e.g., 'updated')")]
    pub status: String,
}

/// Parameters for adding or updating an act.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for adding or updating an act in the narrative")]
pub struct ElicitActParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    pub narrative_id: String,

    /// Act identifier.
    #[schemars(description = "Identifier for the act")]
    pub act_name: String,

    /// Act prompt.
    #[schemars(description = "Prompt text for the act")]
    pub prompt: String,

    /// Model override.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Model to use for this act (overrides default)")]
    pub model: Option<String>,

    /// Temperature override.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Temperature for this act (overrides default)")]
    pub temperature: Option<f64>,
}

/// Result from eliciting an act.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result from adding or updating an act")]
pub struct ElicitActResult {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    pub narrative_id: String,

    /// Act name.
    #[schemars(description = "Name of the act that was added/updated")]
    pub act_name: String,

    /// Status message.
    #[schemars(description = "Status message (e.g., 'created', 'updated')")]
    pub status: String,
}

/// Parameters for finalizing a narrative session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for finalizing a narrative session")]
pub struct FinalizeNarrativeParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session to finalize")]
    pub narrative_id: String,

    /// Whether to validate before finalizing.
    #[serde(default = "default_true")]
    #[schemars(description = "Whether to validate the narrative before finalizing", default = "default_true")]
    pub validate: bool,
}

fn default_true() -> bool {
    true
}

/// Result from finalizing a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result from finalizing a narrative session")]
pub struct FinalizeNarrativeResult {
    /// Whether finalization succeeded.
    #[schemars(description = "True if finalization succeeded")]
    pub success: bool,

    /// TOML representation of the narrative.
    #[schemars(description = "TOML representation of the finalized narrative")]
    pub toml: String,

    /// Validation errors, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Validation errors (if validation was enabled and failed)")]
    pub validation_errors: Option<Vec<String>>,
}

/// Output format for narrative state.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Format for narrative state output")]
pub enum StateFormat {
    /// Brief summary only.
    #[schemars(description = "Brief summary of the narrative")]
    Summary,
    /// Full state information.
    #[schemars(description = "Full state information")]
    Full,
    /// TOML representation.
    #[schemars(description = "TOML representation of the narrative")]
    Toml,
}

impl Default for StateFormat {
    fn default() -> Self {
        Self::Summary
    }
}

/// Summary of narrative state.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Summary of narrative session state")]
pub struct NarrativeStateSummary {
    /// Narrative name (if set).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Name of the narrative (if set)")]
    pub name: Option<String>,

    /// Number of acts.
    #[schemars(description = "Number of acts in the narrative")]
    pub acts_count: usize,

    /// Act names in order.
    #[schemars(description = "List of act names in execution order")]
    pub acts: Vec<String>,

    /// Completeness percentage.
    #[schemars(description = "Completeness percentage (e.g., '66%')")]
    pub completeness: String,

    /// Whether narrative has carousel.
    #[schemars(description = "True if narrative or any act has carousel")]
    pub has_carousel: bool,
}

/// Parameters for getting narrative state.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for querying narrative session state")]
pub struct GetNarrativeStateParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    pub narrative_id: String,

    /// Output format.
    #[serde(default)]
    #[schemars(description = "Output format (summary, full, or toml)", default)]
    pub format: StateFormat,
}

/// Result from getting narrative state.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result containing narrative session state")]
pub struct GetNarrativeStateResult {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    pub narrative_id: String,

    /// State summary.
    #[schemars(description = "Summary of narrative state")]
    pub state: NarrativeStateSummary,

    /// TOML representation (if format is Toml).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "TOML representation (only when format=toml)")]
    pub toml: Option<String>,
}

/// Validation issue severity.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Severity level of a validation issue")]
pub enum ValidationSeverity {
    /// Critical error that prevents execution.
    #[schemars(description = "Critical error that prevents execution")]
    Critical,
    /// High-priority issue.
    #[schemars(description = "High-priority issue")]
    High,
    /// Medium-priority issue.
    #[schemars(description = "Medium-priority issue")]
    Medium,
    /// Low-priority warning.
    #[schemars(description = "Low-priority warning")]
    Low,
}

/// Validation issue.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "A validation issue (error or warning)")]
pub struct ValidationIssue {
    /// Severity level.
    #[schemars(description = "Severity level of this issue")]
    pub severity: ValidationSeverity,

    /// Field or location of issue.
    #[schemars(description = "Field or location where the issue was found")]
    pub field: String,

    /// Issue description.
    #[schemars(description = "Description of the issue")]
    pub message: String,

    /// Suggested fix.
    #[schemars(description = "Suggested fix for the issue")]
    pub suggestion: String,

    /// Whether issue is auto-fixable.
    #[schemars(description = "True if this issue can be automatically fixed")]
    pub auto_fixable: bool,
}

/// Parameters for validating a narrative session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for validating a narrative session")]
pub struct ValidateNarrativeSessionParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session to validate")]
    pub narrative_id: String,

    /// Strict mode flag.
    #[serde(default)]
    #[schemars(description = "Enable strict validation mode", default)]
    pub strict: bool,
}

/// Result from validating a narrative session.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Result from narrative session validation")]
pub struct ValidateNarrativeSessionResult {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    pub narrative_id: String,

    /// Whether validation passed.
    #[schemars(description = "True if narrative is valid")]
    pub is_valid: bool,

    /// Validation errors.
    #[schemars(description = "List of validation errors")]
    pub errors: Vec<ValidationIssue>,

    /// Validation warnings.
    #[schemars(description = "List of validation warnings")]
    pub warnings: Vec<ValidationIssue>,

    /// Completeness percentage.
    #[schemars(description = "Completeness percentage")]
    pub completeness: String,

    /// Count of auto-fixable issues.
    #[schemars(description = "Number of issues that can be automatically fixed")]
    pub auto_fixable_count: usize,
}
