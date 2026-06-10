use botticelli_error::{McpError, McpResult};
use botticelli_interface::ElicitationRegistryOperations;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[tracing::instrument(skip(registry), fields(narrative_id, strict))]
pub async fn validate_narrative<R: ElicitationRegistryOperations<botticelli_narrative::PartialNarrative>>(
    registry: &R,
    input: ValidateNarrativeInput,
) -> McpResult<ValidateNarrativeOutput> {
    tracing::debug!("Validating narrative");

    let narrative_id = Uuid::parse_str(&input.narrative_id)
        .map_err(|e| McpError::invalid_input(format!("Invalid narrative_id: {}", e)))?;

    let partial = registry
        .get_narrative(&narrative_id.to_string())
        .map_err(|e| McpError::execution_failed(e.to_string()))?;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if partial.name.is_none() || partial.name.as_ref().is_none_or(|n| n.is_empty()) {
        errors.push(ValidationIssue {
            severity: Severity::Critical,
            field: "name".to_string(),
            message: "Narrative name is required".to_string(),
            suggestion: "Provide a unique name for this narrative".to_string(),
            auto_fixable: false,
        });
    }

    if partial.description.is_none() || partial.description.as_ref().is_none_or(|d| d.is_empty())
    {
        errors.push(ValidationIssue {
            severity: Severity::High,
            field: "description".to_string(),
            message: "Narrative description is missing".to_string(),
            suggestion: "Add a description explaining the narrative's purpose".to_string(),
            auto_fixable: false,
        });
    }

    if partial.acts.is_empty() {
        errors.push(ValidationIssue {
            severity: Severity::Critical,
            field: "acts".to_string(),
            message: "Narrative has no acts defined".to_string(),
            suggestion: "Add at least one act to define the workflow".to_string(),
            auto_fixable: false,
        });
    }

    for (act_name, act) in &partial.acts {
        if act.prompt.is_empty() {
            errors.push(ValidationIssue {
                severity: Severity::Critical,
                field: format!("acts.{}.prompt", act_name),
                message: format!("Act '{}' has no prompt defined", act_name),
                suggestion: "Add a prompt describing what this act should do".to_string(),
                auto_fixable: true,
            });
        }

        if act.inputs.is_empty() {
            warnings.push(ValidationIssue {
                severity: Severity::Low,
                field: format!("acts.{}.inputs", act_name),
                message: format!("Act '{}' has no inputs", act_name),
                suggestion: "Consider adding inputs if this act needs context".to_string(),
                auto_fixable: false,
            });
        }
    }

    if partial.act_order.is_empty() && !partial.acts.is_empty() {
        warnings.push(ValidationIssue {
            severity: Severity::Medium,
            field: "act_order".to_string(),
            message: "Act order is empty but acts exist".to_string(),
            suggestion: "Define act execution order".to_string(),
            auto_fixable: true,
        });
    }

    if partial.model.is_none() {
        warnings.push(ValidationIssue {
            severity: Severity::Medium,
            field: "model".to_string(),
            message: "No default model specified".to_string(),
            suggestion: "Consider setting a default model".to_string(),
            auto_fixable: true,
        });
    }

    let metadata_complete =
        partial.name.is_some() && partial.description.is_some() && partial.model.is_some();
    let acts_complete =
        !partial.acts.is_empty() && partial.acts.values().all(|act| !act.prompt.is_empty());
    let inputs_partial = partial.acts.values().any(|act| !act.inputs.is_empty());

    let metadata_status = if metadata_complete {
        "complete"
    } else {
        "incomplete"
    };
    let acts_status = if acts_complete {
        "complete"
    } else {
        "incomplete"
    };
    let inputs_status = if inputs_partial { "partial" } else { "none" };

    let mut completeness_score = 0;
    if metadata_complete {
        completeness_score += 33;
    }
    if acts_complete {
        completeness_score += 33;
    }
    if inputs_partial {
        completeness_score += 34;
    }

    Ok(ValidateNarrativeOutput {
        valid: errors.is_empty(),
        errors,
        warnings,
        completeness: CompletenessReport {
            metadata: metadata_status.to_string(),
            acts: acts_status.to_string(),
            inputs: inputs_status.to_string(),
            overall: format!("{}%", completeness_score),
        },
    })
}

#[tracing::instrument(skip(registry), fields(narrative_id))]
#[tracing::instrument(skip(registry), fields(narrative_id))]
pub async fn apply_validation_fixes<R: ElicitationRegistryOperations<botticelli_narrative::PartialNarrative>>(
    registry: &R,
    input: ApplyValidationFixesInput,
) -> McpResult<ApplyValidationFixesOutput> {
    tracing::debug!("Applying validation fixes");

    let narrative_id = Uuid::parse_str(&input.narrative_id)
        .map_err(|e| McpError::invalid_input(format!("Invalid narrative_id: {}", e)))?;

    let mut fixes_applied = Vec::new();

    registry
        .update_narrative(&narrative_id.to_string(), |partial| {
            let fix_all = input.fix_types.contains(&"all".to_string());

            if fix_all || input.fix_types.contains(&"missing_defaults".to_string()) {
                if partial.model.is_none() {
                    partial.model = Some("claude-3-5-sonnet-20241022".to_string());
                    fixes_applied
                        .push("Set default model to claude-3-5-sonnet-20241022".to_string());
                }

                if partial.temperature.is_none() {
                    partial.temperature = Some(0.7);
                    fixes_applied.push("Set default temperature to 0.7".to_string());
                }

                if partial.max_tokens.is_none() {
                    partial.max_tokens = Some(1000);
                    fixes_applied.push("Set default max_tokens to 1000".to_string());
                }
            }

            Ok(())
        })
        .map_err(|e| McpError::execution_failed(e.to_string()))?;

    let validation = validate_narrative(
        registry,
        ValidateNarrativeInput {
            narrative_id: input.narrative_id,
            strict: false,
        },
    )
    .await?;

    Ok(ApplyValidationFixesOutput {
        success: true,
        fixes_applied,
        remaining_errors: validation.errors.len(),
    })
}
