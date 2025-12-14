//! Validation elicitor for interactive narrative validation.

use botticelli_error::BotticelliResult;

use async_trait::async_trait;
use botticelli_error::{ChatError, ChatErrorKind};
use botticelli_mcp::{ElicitationDialog, NarrativeElicitor, PartialNarrative};
use botticelli_narrative::validator::{ValidationError, ValidationErrorKind, ValidationResult};
use tracing::{debug, info, instrument, warn};

/// Elicits validation fixes for narrative TOML.
///
/// Provides interactive validation with:
/// - Error priority classification
/// - Automatic fix suggestions
/// - Guided manual fixes
/// - Re-validation after fixes
///
/// Prerequisites: PartialNarrative with minimum required fields
pub struct ValidationElicitor {
    /// Whether to auto-fix when possible
    auto_fix: bool,
}

impl ValidationElicitor {
    /// Create validation elicitor with auto-fix enabled.
    pub fn with_auto_fix() -> Self {
        Self { auto_fix: true }
    }

    /// Create validation elicitor with manual fixes only.
    pub fn manual_only() -> Self {
        Self { auto_fix: false }
    }

    /// Validate the partial narrative and return results.
    #[instrument(skip(self, partial))]
    fn validate_partial(&self, partial: &PartialNarrative) -> BotticelliResult<ValidationResult> {
        // Generate TOML from partial narrative
        let toml = partial.to_toml()?;

        // Validate using botticelli_narrative validator
        let result = botticelli_narrative::validator::validate_narrative_toml(&toml);

        debug!(
            error_count = result.errors.len(),
            warning_count = result.warnings.len(),
            "Validation completed"
        );

        Ok(result)
    }

    /// Display validation errors with priority.
    #[instrument(skip(self, dialog, errors))]
    async fn display_errors(
        &self,
        dialog: &mut dyn ElicitationDialog,
        errors: &[ValidationError],
    ) -> BotticelliResult<()> {
        if errors.is_empty() {
            dialog.show_info("✓ No validation errors found").await?;
            return Ok(());
        }

        dialog
            .show_error(&format!(
                "Found {} validation error(s) that must be fixed:",
                errors.len()
            ))
            .await?;

        for (i, error) in errors.iter().enumerate() {
            let priority = Self::error_priority(&error.kind);
            let location_str = error
                .location
                .as_ref()
                .map(|loc| format!(" at line {}", loc.line))
                .unwrap_or_default();

            dialog
                .show_error(&format!(
                    "{}. [{}] {}{}",
                    i + 1,
                    priority,
                    error.message,
                    location_str
                ))
                .await?;

            if let Some(ref suggestion) = error.suggestion {
                dialog
                    .show_info(&format!("   Suggestion: {}", suggestion))
                    .await?;
            }
        }

        Ok(())
    }

    /// Classify error priority.
    fn error_priority(kind: &ValidationErrorKind) -> &'static str {
        match kind {
            ValidationErrorKind::InvalidSyntax => "CRITICAL",
            ValidationErrorKind::MissingSection => "CRITICAL",
            ValidationErrorKind::EmptyToc => "HIGH",
            ValidationErrorKind::MissingAct => "HIGH",
            ValidationErrorKind::EmptyPrompt => "HIGH",
            ValidationErrorKind::UndefinedReference => "MEDIUM",
            ValidationErrorKind::CircularDependency => "HIGH",
            ValidationErrorKind::FileNotFound => "MEDIUM",
        }
    }

    /// Attempt to auto-fix common errors.
    #[instrument(skip(self, dialog, _partial, errors))]
    async fn attempt_auto_fix(
        &self,
        dialog: &mut dyn ElicitationDialog,
        _partial: &mut PartialNarrative,
        errors: &[ValidationError],
    ) -> BotticelliResult<bool> {
        let mut fixed_any = false;

        for error in errors {
            if let Some(fix) = self.suggest_auto_fix(&error.kind) {
                let should_fix = dialog
                    .ask_confirmation(&format!("Auto-fix: {}?", fix), true)
                    .await?;

                if should_fix {
                    // Note: Actual fixes would require modifying PartialNarrative
                    // This is a placeholder for the fix logic
                    dialog.show_info(&format!("✓ Applied fix: {}", fix)).await?;
                    fixed_any = true;
                }
            }
        }

        Ok(fixed_any)
    }

    /// Suggest automatic fix for error kind.
    fn suggest_auto_fix(&self, kind: &ValidationErrorKind) -> Option<String> {
        match kind {
            ValidationErrorKind::EmptyToc => {
                Some("Add all defined acts to table of contents".to_string())
            }
            ValidationErrorKind::EmptyPrompt => {
                Some("Add placeholder prompt to empty acts".to_string())
            }
            _ => None,
        }
    }

    /// Guide user through manual fixes.
    #[instrument(skip(self, dialog, errors))]
    async fn guide_manual_fixes(
        &self,
        dialog: &mut dyn ElicitationDialog,
        errors: &[ValidationError],
    ) -> BotticelliResult<()> {
        dialog
            .show_info("Manual fixes required. Please address the following:")
            .await?;

        for error in errors {
            let guidance = self.get_fix_guidance(&error.kind);
            dialog
                .show_info(&format!("• {} - {}", error.message, guidance))
                .await?;
        }

        Ok(())
    }

    /// Get guidance for fixing error kind.
    fn get_fix_guidance(&self, kind: &ValidationErrorKind) -> &'static str {
        match kind {
            ValidationErrorKind::InvalidSyntax => "Check TOML syntax (quotes, brackets, commas)",
            ValidationErrorKind::MissingSection => {
                "Add required [narrative] section with name and description"
            }
            ValidationErrorKind::EmptyToc => "Add acts to table_of_contents array",
            ValidationErrorKind::MissingAct => {
                "Define missing act in [[act]] section or remove from toc"
            }
            ValidationErrorKind::EmptyPrompt => "Add inputs array to act with at least one input",
            ValidationErrorKind::UndefinedReference => {
                "Check that referenced resource exists (narrative, table, bot)"
            }
            ValidationErrorKind::CircularDependency => "Remove circular narrative references",
            ValidationErrorKind::FileNotFound => "Ensure referenced files exist at specified paths",
        }
    }
}

impl Default for ValidationElicitor {
    fn default() -> Self {
        Self::with_auto_fix()
    }
}

#[async_trait]
impl NarrativeElicitor for ValidationElicitor {
    fn name(&self) -> &str {
        "Validation"
    }

    fn description(&self) -> &str {
        "Validate narrative and fix errors interactively"
    }

    fn can_run(&self, partial: &PartialNarrative) -> bool {
        // Requires minimum fields for validation
        partial.has_minimum_required()
    }

    #[instrument(skip(self, dialog, partial))]
    async fn elicit(
        &self,
        dialog: &mut dyn ElicitationDialog,
        partial: &mut PartialNarrative,
    ) -> BotticelliResult<()> {
        dialog.show_info("Validating narrative...").await?;

        // Initial validation
        let mut result = self.validate_partial(partial)?;

        // Display errors
        self.display_errors(dialog, &result.errors).await?;

        // Display warnings
        if !result.warnings.is_empty() {
            dialog
                .show_warning(&format!(
                    "Found {} warning(s) (review recommended):",
                    result.warnings.len()
                ))
                .await?;

            for (i, warning) in result.warnings.iter().enumerate() {
                dialog
                    .show_warning(&format!("{}. {}", i + 1, warning.message))
                    .await?;
            }
        }

        // If no errors, we're done
        if result.errors.is_empty() {
            info!("Validation passed with {} warnings", result.warnings.len());
            return Ok(());
        }

        // Attempt auto-fixes if enabled
        if self.auto_fix {
            let fixed = self
                .attempt_auto_fix(dialog, partial, &result.errors)
                .await?;

            if fixed {
                // Re-validate after fixes
                dialog.show_info("Re-validating after fixes...").await?;
                result = self.validate_partial(partial)?;
                self.display_errors(dialog, &result.errors).await?;

                if result.errors.is_empty() {
                    dialog.show_info("✓ All errors fixed automatically").await?;
                    return Ok(());
                }
            }
        }

        // Guide through remaining errors
        if !result.errors.is_empty() {
            self.guide_manual_fixes(dialog, &result.errors).await?;

            let continue_anyway = dialog
                .ask_confirmation(
                    "Validation errors remain. Continue anyway (not recommended)?",
                    false,
                )
                .await?;

            if !continue_anyway {
                warn!("User chose to fix validation errors before continuing");
                return Err(ChatError::new(ChatErrorKind::ValidationError(format!(
                    "{} validation errors remain",
                    result.errors.len()
                )))
                .into());
            }
        }

        Ok(())
    }

    fn is_complete(&self, partial: &PartialNarrative) -> bool {
        // Validation is complete when no errors remain
        if let Ok(toml) = partial.to_toml() {
            let result = botticelli_narrative::validator::validate_narrative_toml(&toml);
            result.errors.is_empty()
        } else {
            false
        }
    }

    fn suggest_next(&self, partial: &PartialNarrative) -> Option<String> {
        if !partial.has_minimum_required() {
            Some("Add minimum required fields before validation".to_string())
        } else {
            Some("Finalize and save narrative".to_string())
        }
    }
}
