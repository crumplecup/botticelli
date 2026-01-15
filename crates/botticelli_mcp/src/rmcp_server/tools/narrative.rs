//! Narrative management tools.
//!
//! Tools for creating, modifying, validating, and managing narrative workflows.

use super::super::helpers::{apply_modification, generate_narrative_toml, to_mcp_error};
use super::super::server::BotticelliServer;
use crate::tools::NarrativeHelper;
use crate::tools::narrative_validation_helpers::{
    add_helpful_comments, auto_fix_common_issues, format_toml, format_validation_result,
};
use crate::{
    ApplyValidationFixesParams, ApplyValidationFixesResult, CreateNarrativeParams,
    CreateNarrativeResult, FinalizeNarrativeParams, FinalizeNarrativeResult,
    GetNarrativeStateParams, GetNarrativeStateResult, ModifyNarrativeParams,
    ModifyNarrativeResult, NarrativeStateSummary, SaveNarrativeParams,
    SaveNarrativeResult, StateFormat, ValidateNarrativeParams, ValidateNarrativeResult,
    ValidateNarrativeSessionParams, ValidateNarrativeSessionResult, ValidationError,
    ValidationIssue, ValidationLocation, ValidationSeverity, ValidationWarning,
};
use botticelli_narrative::validator::{ValidationConfig, Validator};
use rmcp::handler::server::wrapper::{Json, Parameters};
use std::path::Path;
use tracing::{debug, instrument};

impl BotticelliServer {
    #[instrument(skip(self, description), fields(name, description_len = description.len(), has_model = default_model.is_some()))]
    pub async fn create_narrative(
        &self,
        Parameters(CreateNarrativeParams {
            description,
            name,
            default_model,
            default_temperature,
        }): Parameters<CreateNarrativeParams>,
    ) -> Result<Json<CreateNarrativeResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(name = %name, has_model = default_model.is_some(), "Creating narrative from description");

        // Validate name
        if !NarrativeHelper::is_valid_name(&name) {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!(
                    "Invalid narrative name '{}'. Must be alphanumeric with underscores, starting with a letter",
                    name
                )),
                None,
            ));
        }

        // Generate narrative TOML
        let mut toml = generate_narrative_toml(
            &description,
            &name,
            default_model.as_deref(),
            default_temperature,
        )?;

        // Auto-fix common issues
        let (fixed_toml, fixes_applied) = auto_fix_common_issues(&toml);
        toml = fixed_toml;

        // Format TOML
        toml = format_toml(&toml);

        // Add helpful comments
        let toml_with_comments = add_helpful_comments(&toml);

        // Validate
        let validation = Validator::validate_toml(&toml);

        debug!(
            valid = validation.is_valid(),
            errors = validation.errors().len(),
            warnings = validation.warnings().len(),
            fixes_applied = fixes_applied.len(),
            "Narrative generated and validated"
        );

        // Format validation results
        let validation_json = format_validation_result(&validation);

        // Generate summary
        let act_count = NarrativeHelper::count_acts(&toml);
        let summary = if validation.is_valid() {
            if fixes_applied.is_empty() {
                format!("Created narrative '{}' with {} act(s)", name, act_count)
            } else {
                format!(
                    "Created narrative '{}' with {} act(s) ({} auto-fixes applied)",
                    name,
                    act_count,
                    fixes_applied.len()
                )
            }
        } else {
            format!(
                "Generated narrative has {} error(s) - see validation for details",
                validation.errors().len()
            )
        };

        let result = CreateNarrativeResult::new(
            toml,
            toml_with_comments,
            validation_json,
            summary,
            fixes_applied,
            act_count,
        );

        Ok(Json(result))
    }
    
    #[instrument(skip(self, narrative_toml, modification), fields(toml_len = narrative_toml.len(), modification_len = modification.len(), has_save_path = save_to.is_some()))]
    pub async fn modify_narrative(
        &self,
        Parameters(ModifyNarrativeParams {
            narrative_toml,
            modification,
            save_to,
        }): Parameters<ModifyNarrativeParams>,
    ) -> Result<Json<ModifyNarrativeResult>, rmcp::ErrorData> {
        debug!(modification = %modification, has_save_path = save_to.is_some(), "Modifying narrative");

        // Apply modification
        let (mut modified_toml, change_description) = apply_modification(&narrative_toml, &modification)?;

        // Track changes
        let mut changes = vec![change_description];

        // Auto-fix common issues
        let (fixed_toml, fixes_applied) = auto_fix_common_issues(&modified_toml);
        if !fixes_applied.is_empty() {
            modified_toml = fixed_toml;
            changes.extend(fixes_applied.iter().map(|f| format!("Auto-fix: {}", f)));
        }

        // Format TOML
        modified_toml = format_toml(&modified_toml);

        // Validate
        let validation = Validator::validate_toml(&modified_toml);

        debug!(
            valid = validation.is_valid(),
            changes = changes.len(),
            auto_fixes = fixes_applied.len(),
            "Narrative modified and validated"
        );

        // Optionally save to file
        let mut saved_to = None;
        if let Some(path) = save_to {
            tokio::fs::write(&path, &modified_toml)
                .await
                .map_err(|e| to_mcp_error(e, "Failed to save file"))?;
            saved_to = Some(path);
            debug!(
                path = saved_to.as_ref().unwrap(),
                "Saved modified narrative to file"
            );
        }

        // Format validation results
        let validation_json = format_validation_result(&validation);

        let result = ModifyNarrativeResult::new(modified_toml, validation_json, changes, saved_to);
        Ok(Json(result))
    }
    
    #[instrument(skip(self, narrative_toml), fields(toml_len = narrative_toml.len(), path = %file_path, overwrite))]
    pub async fn save_narrative(
        &self,
        Parameters(SaveNarrativeParams {
            narrative_toml,
            file_path,
            overwrite,
        }): Parameters<SaveNarrativeParams>,
    ) -> Result<Json<SaveNarrativeResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(path = %file_path, overwrite, "Saving narrative to file");

        // Validate path
        let path = Path::new(&file_path);

        // Check extension
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("File path must end with .toml extension"),
                None,
            ));
        }

        // Check if file exists
        let existed = path.exists();
        if existed && !overwrite {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!(
                    "File '{}' already exists. Set overwrite=true to replace it",
                    file_path
                )),
                None,
            ));
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| to_mcp_error(e, "Failed to create directories"))?;
                debug!(path = ?parent, "Created parent directories");
            }
        }

        // Write file
        tokio::fs::write(path, &narrative_toml)
            .await
            .map_err(|e| to_mcp_error(e, "Failed to write file"))?;

        debug!(path = %file_path, "Narrative saved to file");

        // Get absolute path for response
        let absolute_path = std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.clone());

        let result = SaveNarrativeResult::new(absolute_path, narrative_toml.len(), existed);
        Ok(Json(result))
    }
    pub async fn validate_narrative(
        &self,
        Parameters(ValidateNarrativeParams {
            content,
            file_path,
            validate_files,
            validate_models,
            warn_unused,
            strict,
        }): Parameters<ValidateNarrativeParams>,
    ) -> Result<Json<ValidateNarrativeResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;
        use std::path::PathBuf;

        debug!(
            ?file_path,
            has_content = content.is_some(),
            "Validating narrative"
        );

        // Get TOML content
        let toml_content = if let Some(c) = content {
            c
        } else if let Some(ref path) = file_path {
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| to_mcp_error(e, &format!("Failed to read file '{}'", path)))?
        } else {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("Either 'content' or 'file_path' must be provided"),
                None,
            ));
        };

        // Configure validation
        let mut config = ValidationConfig::default();
        config = config
            .with_validate_nested_narratives(validate_files)
            .with_validate_media_files(validate_files)
            .with_warn_unknown_models(validate_models)
            .with_warn_unused_resources(warn_unused);

        if let Some(p) =
            file_path.and_then(|p| PathBuf::from(p).parent().map(|parent| parent.to_path_buf()))
        {
            config = config.with_base_dir(Some(p));
        }

        // Validate
        let result = Validator::validate_toml_with_config(&toml_content, &config);

        // Convert errors
        let errors: Vec<ValidationError> = result
            .errors()
            .iter()
            .map(|e| ValidationError {
                kind: format!("{:?}", e.kind()),
                message: e.message().clone(),
                suggestion: e.suggestion().clone(),
                location: e.location().as_ref().map(|loc| ValidationLocation {
                    line: *loc.line(),
                    column: *loc.column(),
                    section: loc.section().clone(),
                }),
            })
            .collect();

        // Convert warnings
        let warnings: Vec<ValidationWarning> = result
            .warnings()
            .iter()
            .map(|w| ValidationWarning {
                kind: format!("{:?}", w.kind()),
                message: w.message().clone(),
                location: w.location().as_ref().map(|loc| ValidationLocation {
                    line: *loc.line(),
                    column: *loc.column(),
                    section: loc.section().clone(),
                }),
            })
            .collect();

        let is_valid = result.is_valid();
        let has_warnings = !result.warnings().is_empty();
        let valid = is_valid && (!strict || !has_warnings);

        debug!(
            valid,
            errors = errors.len(),
            warnings = warnings.len(),
            "Validation complete"
        );

        Ok(Json(ValidateNarrativeResult::new(valid, errors, warnings)))
    }
    
    #[instrument(skip(self), fields(narrative_id, validate))]
    pub async fn finalize_narrative(
        &self,
        Parameters(FinalizeNarrativeParams {
            narrative_id,
            validate,
        }): Parameters<FinalizeNarrativeParams>,
    ) -> Result<Json<FinalizeNarrativeResult>, rmcp::ErrorData> {
        debug!(narrative_id, validate, "Finalizing narrative");

        // Get narrative
        let partial = self
            .narrative_registry
            .get(&narrative_id)
            .map_err(|e| to_mcp_error(e, "Narrative not found"))?;

        // Convert to TOML
        let toml = toml::to_string_pretty(&partial)
            .map_err(|e| to_mcp_error(e, "Failed to serialize to TOML"))?;

        // Validate if requested
        let validation_errors = if validate {
            let config = ValidationConfig::default();
            let result = Validator::validate_toml_with_config(&toml, &config);

            if !result.is_valid() {
                Some(
                    result
                        .errors()
                        .iter()
                        .map(|e| e.message().clone())
                        .collect(),
                )
            } else {
                None
            }
        } else {
            None
        };

        let success = validation_errors.is_none();

        // Remove from registry (session complete)
        self.narrative_registry.remove(&narrative_id);

        debug!(narrative_id, success, "Narrative finalized");

        Ok(Json(FinalizeNarrativeResult {
            success,
            toml,
            validation_errors,
        }))
    }
    
    #[instrument(skip(self), fields(narrative_id, format = ?format))]
    pub async fn get_narrative_state(
        &self,
        Parameters(GetNarrativeStateParams {
            narrative_id,
            format,
        }): Parameters<GetNarrativeStateParams>,
    ) -> Result<Json<GetNarrativeStateResult>, rmcp::ErrorData> {
        debug!(narrative_id, ?format, "Getting narrative state");

        // Get narrative
        let partial = self
            .narrative_registry
            .get(&narrative_id)
            .map_err(|e| to_mcp_error(e, "Narrative not found"))?;

        // Calculate state
        let acts_count = partial.acts().len();
        let acts: Vec<String> = partial.act_order().clone();
        let has_carousel =
            partial.carousel().is_some() || partial.acts().values().any(|act| act.carousel().is_some());

        let metadata_complete =
            partial.name().is_some() && partial.description().is_some() && partial.model().is_some();
        let acts_complete =
            !partial.acts().is_empty() && partial.acts().values().all(|act| !act.prompt().is_empty());
        let inputs_partial = partial.acts().values().any(|act| !act.inputs().is_empty());

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

        // Generate TOML if requested
        let toml = if matches!(format, StateFormat::Toml) {
            Some(
                toml::to_string_pretty(&partial)
                    .map_err(|e| to_mcp_error(e, "Failed to convert to TOML"))?,
            )
        } else {
            None
        };

        debug!(
            narrative_id,
            completeness = %completeness_score,
            acts_count,
            "Narrative state retrieved"
        );

        Ok(Json(GetNarrativeStateResult {
            narrative_id,
            state: NarrativeStateSummary {
                name: partial.name().clone(),
                acts_count,
                acts,
                completeness: format!("{}%", completeness_score),
                has_carousel,
            },
            toml,
        }))
    }
    
    #[instrument(skip(self), fields(narrative_id, strict))]
    pub async fn validate_narrative_session(
        &self,
        Parameters(ValidateNarrativeSessionParams {
            narrative_id,
            strict,
        }): Parameters<ValidateNarrativeSessionParams>,
    ) -> Result<Json<ValidateNarrativeSessionResult>, rmcp::ErrorData> {
        debug!(narrative_id, strict, "Validating narrative session");

        // Get narrative
        let partial = self
            .narrative_registry
            .get(&narrative_id)
            .map_err(|e| to_mcp_error(e, "Narrative not found"))?;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate name
        if partial.name().is_none() || partial.name().as_ref().is_some_and(|n| n.is_empty()) {
            errors.push(ValidationIssue {
                severity: ValidationSeverity::Critical,
                field: "name".to_string(),
                message: "Narrative name is required".to_string(),
                suggestion: "Provide a unique name for this narrative".to_string(),
                auto_fixable: false,
            });
        }

        // Validate description
        if partial.description().is_none()
            || partial.description().as_ref().is_some_and(|d| d.is_empty())
        {
            errors.push(ValidationIssue {
                severity: ValidationSeverity::High,
                field: "description".to_string(),
                message: "Narrative description is missing".to_string(),
                suggestion: "Add a description explaining what this narrative does".to_string(),
                auto_fixable: false,
            });
        }

        // Validate model
        if partial.model().is_none() {
            warnings.push(ValidationIssue {
                severity: ValidationSeverity::Medium,
                field: "model".to_string(),
                message: "Default model not specified".to_string(),
                suggestion: "Set a default model (e.g., 'gemini-2.0-flash-exp')".to_string(),
                auto_fixable: true,
            });
        }

        // Validate acts
        if partial.acts().is_empty() {
            errors.push(ValidationIssue {
                severity: ValidationSeverity::Critical,
                field: "acts".to_string(),
                message: "Narrative has no acts".to_string(),
                suggestion: "Add at least one act to the narrative".to_string(),
                auto_fixable: false,
            });
        } else {
            for (act_name, act) in partial.acts() {
                if act.prompt().is_empty() {
                    errors.push(ValidationIssue {
                        severity: ValidationSeverity::High,
                        field: format!("acts.{}.prompt", act_name),
                        message: format!("Act '{}' has empty prompt", act_name),
                        suggestion: "Provide a prompt for this act".to_string(),
                        auto_fixable: false,
                    });
                }

                if strict && act.model().is_none() && partial.model().is_none() {
                    warnings.push(ValidationIssue {
                        severity: ValidationSeverity::Low,
                        field: format!("acts.{}.model", act_name),
                        message: format!("Act '{}' has no model specified", act_name),
                        suggestion: "Set model for act or narrative default".to_string(),
                        auto_fixable: true,
                    });
                }
            }
        }

        // Calculate completeness
        let metadata_complete =
            partial.name().is_some() && partial.description().is_some() && partial.model().is_some();
        let acts_complete =
            !partial.acts().is_empty() && partial.acts().values().all(|act| !act.prompt().is_empty());

        let mut completeness_score = 0;
        if metadata_complete {
            completeness_score += 50;
        }
        if acts_complete {
            completeness_score += 50;
        }

        let auto_fixable_count = errors
            .iter()
            .chain(warnings.iter())
            .filter(|issue| issue.auto_fixable)
            .count();

        let is_valid = errors.is_empty();

        debug!(
            narrative_id,
            is_valid,
            errors = errors.len(),
            warnings = warnings.len(),
            "Validation complete"
        );

        Ok(Json(ValidateNarrativeSessionResult {
            narrative_id,
            is_valid,
            errors,
            warnings,
            completeness: format!("{}%", completeness_score),
            auto_fixable_count,
        }))
    }
    
    #[instrument(skip(self, fix_types), fields(narrative_id, fix_count = fix_types.len(), confirm))]
    pub async fn apply_validation_fixes(
        &self,
        Parameters(ApplyValidationFixesParams {
            narrative_id,
            fix_types,
            confirm,
        }): Parameters<ApplyValidationFixesParams>,
    ) -> Result<Json<ApplyValidationFixesResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(
            narrative_id,
            ?fix_types,
            confirm,
            "Applying validation fixes"
        );

        if !confirm {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("Must set confirm=true to apply fixes"),
                None,
            ));
        }

        // Get narrative
        let mut partial = self.narrative_registry.get(&narrative_id).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative not found: {}", e)),
                None,
            )
        })?;

        let mut fixes_applied = Vec::new();

        // Apply fixes based on type
        let fix_all = fix_types.contains(&"all".to_string());

        if fix_all || fix_types.contains(&"missing_defaults".to_string()) {
            if partial.model().is_none() {
                partial.with_model(Some("gemini-2.0-flash-exp".to_string()));
                fixes_applied.push("Set default model to gemini-2.0-flash-exp".to_string());
            }

            if partial.temperature().is_none() {
                partial.with_temperature(Some(0.7));
                fixes_applied.push("Set default temperature to 0.7".to_string());
            }

            if partial.max_tokens().is_none() {
                partial.with_max_tokens(Some(1000));
                fixes_applied.push("Set default max_tokens to 1000".to_string());
            }
        }

        // Update narrative in registry
        self.narrative_registry.add(partial);

        // Count remaining errors by validating
        let remaining_errors = {
            let partial = self
                .narrative_registry
                .get(&narrative_id)
                .map_err(|e| to_mcp_error(e, "Failed to retrieve updated narrative"))?;

            let mut errors = 0;

            if partial.name().is_none() || partial.name().as_ref().is_some_and(|n| n.is_empty()) {
                errors += 1;
            }

            if partial.description().is_none()
                || partial.description().as_ref().is_some_and(|d| d.is_empty())
            {
                errors += 1;
            }

            if partial.acts().is_empty() {
                errors += 1;
            } else {
                for act in partial.acts().values() {
                    if act.prompt().is_empty() {
                        errors += 1;
                    }
                }
            }

            errors
        };

        debug!(
            narrative_id,
            fixes_count = fixes_applied.len(),
            remaining_errors,
            "Validation fixes applied"
        );

        Ok(Json(ApplyValidationFixesResult {
            success: true,
            fixes_applied,
            remaining_errors,
        }))
    }
}
