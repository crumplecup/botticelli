//! Session-based narrative elicitation tool types.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Parameters for creating a narrative session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
#[schemars(description = "Parameters for initializing a new narrative creation session")]
pub struct CreateNarrativeSessionParams {
    /// User's description of what they want to create.
    #[schemars(description = "User's description of the narrative they want to create")]
    description: String,
}

impl CreateNarrativeSessionParams {
    /// Create new create narrative session parameters.
    #[instrument]
    pub fn new(description: String) -> Self {
        Self { description }
    }
}

/// Analysis of the user's description.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
#[schemars(description = "Analysis of the user's narrative description")]
pub struct NarrativeAnalysis {
    /// Names of detected acts.
    #[schemars(description = "List of detected act names")]
    detected_acts: Vec<String>,

    /// Complexity assessment.
    #[schemars(description = "Complexity assessment: simple, moderate, or complex")]
    complexity: String,

    /// Number of acts detected.
    #[schemars(description = "Number of acts detected in the description")]
    act_count: usize,
}

impl NarrativeAnalysis {
    /// Create new narrative analysis.
    #[instrument]
    pub fn new(detected_acts: Vec<String>, complexity: String, act_count: usize) -> Self {
        Self {
            detected_acts,
            complexity,
            act_count,
        }
    }
}

/// Result from creating a narrative session.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
#[schemars(description = "Result from creating a new narrative session")]
pub struct CreateNarrativeSessionResult {
    /// UUID of the created session.
    #[schemars(description = "UUID of the created narrative session")]
    narrative_id: String,

    /// Suggested name for the narrative.
    #[schemars(description = "Suggested name based on the description")]
    suggested_name: String,

    /// Analysis of the description.
    #[schemars(description = "Analysis of the user's description")]
    analysis: NarrativeAnalysis,
}

impl CreateNarrativeSessionResult {
    /// Create new create narrative session result.
    #[instrument(skip(analysis))]
    pub fn new(narrative_id: String, suggested_name: String, analysis: NarrativeAnalysis) -> Self {
        Self {
            narrative_id,
            suggested_name,
            analysis,
        }
    }
}

/// Parameters for setting narrative metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, derive_builder::Builder)]
#[schemars(description = "Parameters for setting or updating narrative metadata")]
#[builder(setter(into))]
pub struct ElicitMetadataParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    narrative_id: String,

    /// Narrative name.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Name of the narrative")]
    name: Option<String>,

    /// Narrative description.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Description of the narrative")]
    description: Option<String>,

    /// Default model for acts.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Default model to use for acts")]
    default_model: Option<String>,

    /// Default temperature (0.0-1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Default temperature (0.0-1.0)")]
    default_temperature: Option<f64>,
}

impl ElicitMetadataParams {
    /// Create a builder for elicit metadata parameters.
    #[instrument]
    pub fn builder() -> ElicitMetadataParamsBuilder {
        ElicitMetadataParamsBuilder::default()
    }
}

/// Result from eliciting metadata.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
#[schemars(description = "Result from updating narrative metadata")]
pub struct ElicitMetadataResult {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    narrative_id: String,

    /// Status message.
    #[schemars(description = "Status message (e.g., 'updated')")]
    status: String,
}

impl ElicitMetadataResult {
    /// Create new elicit metadata result.
    #[instrument]
    pub fn new(narrative_id: String, status: String) -> Self {
        Self {
            narrative_id,
            status,
        }
    }
}

/// Parameters for adding or updating an act.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, derive_builder::Builder)]
#[schemars(description = "Parameters for adding or updating an act in the narrative")]
#[builder(setter(into))]
pub struct ElicitActParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    narrative_id: String,

    /// Act identifier.
    #[schemars(description = "Identifier for the act")]
    act_name: String,

    /// Act prompt.
    #[schemars(description = "Prompt text for the act")]
    prompt: String,

    /// Model override.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Model to use for this act (overrides default)")]
    model: Option<String>,

    /// Temperature override.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Temperature for this act (overrides default)")]
    temperature: Option<f64>,
}

impl ElicitActParams {
    /// Create a builder for elicit act parameters.
    #[instrument]
    pub fn builder() -> ElicitActParamsBuilder {
        ElicitActParamsBuilder::default()
    }
}

/// Result from eliciting an act.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
#[schemars(description = "Result from adding or updating an act")]
pub struct ElicitActResult {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    narrative_id: String,

    /// Act name.
    #[schemars(description = "Name of the act that was added/updated")]
    act_name: String,

    /// Status message.
    #[schemars(description = "Status message (e.g., 'created', 'updated')")]
    status: String,
}

impl ElicitActResult {
    /// Create new elicit act result.
    #[instrument]
    pub fn new(narrative_id: String, act_name: String, status: String) -> Self {
        Self {
            narrative_id,
            act_name,
            status,
        }
    }
}

/// Parameters for finalizing a narrative session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
#[schemars(description = "Parameters for finalizing a narrative session")]
pub struct FinalizeNarrativeParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session to finalize")]
    narrative_id: String,

    /// Whether to validate before finalizing.
    #[serde(default = "default_true")]
    #[schemars(
        description = "Whether to validate the narrative before finalizing",
        default = "default_true"
    )]
    validate: bool,
}

impl FinalizeNarrativeParams {
    /// Create new finalize narrative parameters.
    #[instrument]
    pub fn new(narrative_id: String, validate: bool) -> Self {
        Self {
            narrative_id,
            validate,
        }
    }
}

#[tracing::instrument]
fn default_true() -> bool {
    true
}

/// Result from finalizing a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
#[schemars(description = "Result from finalizing a narrative session")]
pub struct FinalizeNarrativeResult {
    /// Whether finalization succeeded.
    #[schemars(description = "True if finalization succeeded")]
    success: bool,

    /// TOML representation of the narrative.
    #[schemars(description = "TOML representation of the finalized narrative")]
    toml: String,

    /// Validation errors, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Validation errors (if validation was enabled and failed)")]
    validation_errors: Option<Vec<String>>,
}

impl FinalizeNarrativeResult {
    /// Create new finalize narrative result.
    #[instrument(skip(validation_errors))]
    pub fn new(success: bool, toml: String, validation_errors: Option<Vec<String>>) -> Self {
        Self {
            success,
            toml,
            validation_errors,
        }
    }
}

/// Output format for narrative state.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Format for narrative state output")]
pub enum StateFormat {
    /// Brief summary only.
    #[default]
    #[schemars(description = "Brief summary of the narrative")]
    Summary,
    /// Full state information.
    #[schemars(description = "Full state information")]
    Full,
    /// TOML representation.
    #[schemars(description = "TOML representation of the narrative")]
    Toml,
}

/// Summary of narrative state.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Summary of narrative session state")]
pub struct NarrativeStateSummary {
    /// Narrative name (if set).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Name of the narrative (if set)")]
    name: Option<String>,

    /// Number of acts.
    #[schemars(description = "Number of acts in the narrative")]
    acts_count: usize,

    /// Act names in order.
    #[schemars(description = "List of act names in execution order")]
    acts: Vec<String>,

    /// Completeness percentage.
    #[schemars(description = "Completeness percentage (e.g., '66%')")]
    completeness: String,

    /// Whether narrative has carousel.
    #[schemars(description = "True if narrative or any act has carousel")]
    has_carousel: bool,
}

impl NarrativeStateSummary {
    /// Creates a new narrative state summary.
    #[instrument]
    pub fn new(
        name: Option<String>,
        acts_count: usize,
        acts: Vec<String>,
        completeness: String,
        has_carousel: bool,
    ) -> Self {
        Self {
            name,
            acts_count,
            acts,
            completeness,
            has_carousel,
        }
    }
}

/// Parameters for getting narrative state.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Parameters for querying narrative session state")]
pub struct GetNarrativeStateParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    narrative_id: String,

    /// Output format.
    #[serde(default)]
    #[schemars(description = "Output format (summary, full, or toml)", default)]
    format: StateFormat,
}

impl GetNarrativeStateParams {
    /// Creates new parameters for getting narrative state.
    #[instrument]
    pub fn new(narrative_id: String, format: StateFormat) -> Self {
        Self {
            narrative_id,
            format,
        }
    }
}

/// Result from getting narrative state.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Result containing narrative session state")]
pub struct GetNarrativeStateResult {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    narrative_id: String,

    /// State summary.
    #[schemars(description = "Summary of narrative state")]
    state: NarrativeStateSummary,

    /// TOML representation (if format is Toml).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "TOML representation (only when format=toml)")]
    toml: Option<String>,
}

impl GetNarrativeStateResult {
    /// Creates a new narrative state result.
    #[instrument(skip(state))]
    pub fn new(narrative_id: String, state: NarrativeStateSummary, toml: Option<String>) -> Self {
        Self {
            narrative_id,
            state,
            toml,
        }
    }
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
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "A validation issue (error or warning)")]
pub struct ValidationIssue {
    /// Severity level.
    #[schemars(description = "Severity level of this issue")]
    severity: ValidationSeverity,

    /// Field or location of issue.
    #[schemars(description = "Field or location where the issue was found")]
    field: String,

    /// Issue description.
    #[schemars(description = "Description of the issue")]
    message: String,

    /// Suggested fix.
    #[schemars(description = "Suggested fix for the issue")]
    suggestion: String,

    /// Whether issue is auto-fixable.
    #[schemars(description = "True if this issue can be automatically fixed")]
    auto_fixable: bool,
}

impl ValidationIssue {
    /// Creates a new validation issue.
    #[instrument]
    pub fn new(
        severity: ValidationSeverity,
        field: String,
        message: String,
        suggestion: String,
        auto_fixable: bool,
    ) -> Self {
        Self {
            severity,
            field,
            message,
            suggestion,
            auto_fixable,
        }
    }
}

/// Parameters for validating a narrative session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Parameters for validating a narrative session")]
pub struct ValidateNarrativeSessionParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session to validate")]
    narrative_id: String,

    /// Strict mode flag.
    #[serde(default)]
    #[schemars(description = "Enable strict validation mode", default)]
    strict: bool,
}

impl ValidateNarrativeSessionParams {
    /// Creates new parameters for validating a narrative session.
    #[instrument]
    pub fn new(narrative_id: String, strict: bool) -> Self {
        Self {
            narrative_id,
            strict,
        }
    }
}

/// Result from validating a narrative session.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Result from narrative session validation")]
pub struct ValidateNarrativeSessionResult {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    narrative_id: String,

    /// Whether validation passed.
    #[schemars(description = "True if narrative is valid")]
    is_valid: bool,

    /// Validation errors.
    #[schemars(description = "List of validation errors")]
    errors: Vec<ValidationIssue>,

    /// Validation warnings.
    #[schemars(description = "List of validation warnings")]
    warnings: Vec<ValidationIssue>,

    /// Completeness percentage.
    #[schemars(description = "Completeness percentage")]
    completeness: String,

    /// Count of auto-fixable issues.
    #[schemars(description = "Number of issues that can be automatically fixed")]
    auto_fixable_count: usize,
}

impl ValidateNarrativeSessionResult {
    /// Creates a new validation result.
    #[instrument(skip(errors, warnings))]
    pub fn new(
        narrative_id: String,
        is_valid: bool,
        errors: Vec<ValidationIssue>,
        warnings: Vec<ValidationIssue>,
        completeness: String,
        auto_fixable_count: usize,
    ) -> Self {
        Self {
            narrative_id,
            is_valid,
            errors,
            warnings,
            completeness,
            auto_fixable_count,
        }
    }
}

/// Parameters for applying automated validation fixes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Parameters for applying automated fixes to validation issues")]
pub struct ApplyValidationFixesParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session to fix")]
    narrative_id: String,

    /// Types of fixes to apply.
    #[schemars(description = "Types of fixes to apply (e.g., 'missing_defaults', 'all')")]
    fix_types: Vec<String>,

    /// Confirm application.
    #[serde(default)]
    #[schemars(description = "Confirm application of fixes", default)]
    confirm: bool,
}

impl ApplyValidationFixesParams {
    /// Creates new parameters for applying validation fixes.
    #[instrument]
    pub fn new(narrative_id: String, fix_types: Vec<String>, confirm: bool) -> Self {
        Self {
            narrative_id,
            fix_types,
            confirm,
        }
    }
}

/// Result from applying validation fixes.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Result from applying automated validation fixes")]
pub struct ApplyValidationFixesResult {
    /// Whether fixes were successfully applied.
    #[schemars(description = "True if fixes were successfully applied")]
    success: bool,

    /// List of fixes applied.
    #[schemars(description = "List of fixes that were applied")]
    fixes_applied: Vec<String>,

    /// Number of errors remaining.
    #[schemars(description = "Number of errors remaining after fixes")]
    remaining_errors: usize,
}

impl ApplyValidationFixesResult {
    /// Creates a new validation fixes result.
    #[instrument]
    pub fn new(success: bool, fixes_applied: Vec<String>, remaining_errors: usize) -> Self {
        Self {
            success,
            fixes_applied,
            remaining_errors,
        }
    }
}

/// Level at which carousel operates.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Level at which carousel configuration applies")]
pub enum CarouselLevel {
    /// Narrative-level carousel.
    #[schemars(description = "Apply carousel to entire narrative")]
    Narrative,
    /// Act-level carousel.
    #[schemars(description = "Apply carousel to specific act")]
    Act,
}

/// Summary of carousel configuration.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Summary of carousel configuration")]
pub struct CarouselSummary {
    /// Level (narrative or act).
    #[schemars(description = "Level: 'narrative' or 'act'")]
    level: String,

    /// Act name (for act-level carousel).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Act name (for act-level carousel)")]
    act_name: Option<String>,

    /// Number of iterations.
    #[schemars(description = "Number of carousel iterations")]
    iterations: u32,

    /// Estimated total tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Estimated total tokens across all iterations")]
    estimated_total_tokens: Option<u32>,

    /// Budget warnings.
    #[schemars(description = "List of budget-related warnings")]
    budget_warnings: Vec<String>,
}

impl CarouselSummary {
    /// Creates a new carousel summary.
    #[instrument]
    pub fn new(
        level: String,
        act_name: Option<String>,
        iterations: u32,
        estimated_total_tokens: Option<u32>,
        budget_warnings: Vec<String>,
    ) -> Self {
        Self {
            level,
            act_name,
            iterations,
            estimated_total_tokens,
            budget_warnings,
        }
    }
}

/// Parameters for creating carousel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Parameters for creating carousel configuration")]
pub struct ElicitCarouselParams {
    /// Session UUID.
    #[schemars(description = "UUID of the narrative session")]
    narrative_id: String,

    /// Carousel level.
    #[schemars(description = "Level at which carousel operates (narrative or act)")]
    level: CarouselLevel,

    /// Act name (required for act-level carousel).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Act name (required when level is 'act')")]
    act_name: Option<String>,

    /// Number of iterations.
    #[schemars(description = "Number of carousel iterations")]
    iterations: u32,

    /// Continue on error flag.
    #[serde(default)]
    #[schemars(description = "Continue processing iterations on error", default)]
    continue_on_error: bool,

    /// Estimated tokens per iteration.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Estimated tokens per iteration for budget planning")]
    estimated_tokens_per_iteration: Option<u32>,

    /// Budget multiplier for warnings.
    #[serde(default = "default_budget_multiplier")]
    #[schemars(
        description = "Budget multiplier for warnings (default: 2.0)",
        default = "default_budget_multiplier"
    )]
    budget_multiplier: f64,
}

impl ElicitCarouselParams {
    /// Creates new parameters for carousel configuration.
    #[instrument]
    pub fn new(
        narrative_id: String,
        level: CarouselLevel,
        act_name: Option<String>,
        iterations: u32,
        continue_on_error: bool,
        estimated_tokens_per_iteration: Option<u32>,
        budget_multiplier: f64,
    ) -> Self {
        Self {
            narrative_id,
            level,
            act_name,
            iterations,
            continue_on_error,
            estimated_tokens_per_iteration,
            budget_multiplier,
        }
    }
}

#[tracing::instrument]
fn default_budget_multiplier() -> f64 {
    2.0
}

/// Result from creating carousel configuration.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters)]
#[schemars(description = "Result from creating carousel configuration")]
pub struct ElicitCarouselResult {
    /// Whether carousel was successfully configured.
    #[schemars(description = "True if carousel configuration succeeded")]
    success: bool,

    /// Carousel configuration summary.
    #[schemars(description = "Summary of the configured carousel")]
    carousel_config: CarouselSummary,
}

impl ElicitCarouselResult {
    /// Creates a new carousel result.
    #[instrument(skip(carousel_config))]
    pub fn new(success: bool, carousel_config: CarouselSummary) -> Self {
        Self {
            success,
            carousel_config,
        }
    }
}
