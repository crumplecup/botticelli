//! Narrative management tools.
//!
//! Tools for creating, modifying, validating, and managing narrative workflows.

use crate::rmcp_server::BotticelliServer;
use crate::rmcp_server::helpers::{apply_modification, generate_narrative_toml, to_mcp_error};
use crate::tools::NarrativeHelper;
use crate::tools::narrative_validation_helpers::{
    add_helpful_comments, auto_fix_common_issues, format_toml, format_validation_result,
};
use crate::{
    ApplyValidationFixesParams, ApplyValidationFixesResult, CreateNarrativeParams,
    CreateNarrativeResult, FinalizeNarrativeParams, FinalizeNarrativeResult,
    GetNarrativeStateParams, GetNarrativeStateResult, ModifyNarrativeParams, ModifyNarrativeResult,
    NarrativeStateSummary, SaveNarrativeParams, SaveNarrativeResult, StateFormat,
    ValidateNarrativeParams, ValidateNarrativeResult, ValidateNarrativeSessionParams,
    ValidateNarrativeSessionResult, ValidationError, ValidationIssue, ValidationLocation,
    ValidationSeverity, ValidationWarning,
};
use botticelli_narrative::validator::{ValidationConfig, Validator};
use botticelli_narrative::{MultiNarrative, Narrative, StateManager};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::ErrorCode;
use rmcp::tool;
use rmcp::tool_router;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};

impl BotticelliServer {
    /// Create a new narrative from a description using LLM analysis.
    #[instrument(skip(self, params), fields(name = params.name(), description_len = params.description().len(), has_model = params.default_model().is_some()))]
    pub async fn create_narrative(
        &self,
        Parameters(params): Parameters<CreateNarrativeParams>,
    ) -> Result<Json<CreateNarrativeResult>, rmcp::ErrorData> {
        let description = params.description().clone();
        let name = params.name().clone();
        let default_model = params.default_model().clone();
        let default_temperature = params.default_temperature().clone();

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

    /// Modify an existing narrative TOML with natural language instructions.
    #[instrument(skip(self, params), fields(toml_len = params.narrative_toml().len(), modification_len = params.modification().len(), has_save_path = params.save_to().is_some()))]
    pub async fn modify_narrative(
        &self,
        Parameters(params): Parameters<ModifyNarrativeParams>,
    ) -> Result<Json<ModifyNarrativeResult>, rmcp::ErrorData> {
        let narrative_toml = params.narrative_toml().clone();
        let modification = params.modification().clone();
        let save_to = params.save_to().clone();

        debug!(modification = %modification, has_save_path = save_to.is_some(), "Modifying narrative");

        // Apply modification
        let (mut modified_toml, change_description) =
            apply_modification(&narrative_toml, &modification)?;

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

    /// Save narrative TOML content to a file.
    #[instrument(skip(self, params), fields(toml_len = params.narrative_toml().len(), path = %params.file_path(), overwrite = params.overwrite()))]
    pub async fn save_narrative(
        &self,
        Parameters(params): Parameters<SaveNarrativeParams>,
    ) -> Result<Json<SaveNarrativeResult>, rmcp::ErrorData> {
        let narrative_toml = params.narrative_toml().clone();
        let file_path = params.file_path().clone();
        let overwrite = *params.overwrite();

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

    /// Validate narrative TOML file structure and content.
    #[instrument(skip(self, params), fields(
        has_content = params.content().is_some(),
        has_file_path = params.file_path().is_some()
    ))]
    pub async fn validate_narrative(
        &self,
        Parameters(params): Parameters<ValidateNarrativeParams>,
    ) -> Result<Json<ValidateNarrativeResult>, rmcp::ErrorData> {
        let content = params.content().clone();
        let file_path = params.file_path().clone();
        let validate_files = *params.validate_files();
        let validate_models = *params.validate_models();
        let warn_unused = *params.warn_unused();
        let strict = *params.strict();

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
            .map(|e| {
                ValidationError::new(
                    format!("{:?}", e.kind()),
                    e.message().clone(),
                    e.suggestion().clone(),
                    e.location().as_ref().map(|loc| {
                        ValidationLocation::new(*loc.line(), *loc.column(), loc.section().clone())
                    }),
                )
            })
            .collect();

        // Convert warnings
        let warnings: Vec<ValidationWarning> = result
            .warnings()
            .iter()
            .map(|w| {
                ValidationWarning::new(
                    format!("{:?}", w.kind()),
                    w.message().clone(),
                    w.location().as_ref().map(|loc| {
                        ValidationLocation::new(*loc.line(), *loc.column(), loc.section().clone())
                    }),
                )
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

    /// Finalize a narrative session and convert to TOML.
    #[instrument(skip(self, params), fields(narrative_id = params.narrative_id(), validate = params.validate()))]
    pub async fn finalize_narrative(
        &self,
        Parameters(params): Parameters<FinalizeNarrativeParams>,
    ) -> Result<Json<FinalizeNarrativeResult>, rmcp::ErrorData> {
        let narrative_id = params.narrative_id().clone();
        let validate = *params.validate();

        debug!(narrative_id, validate, "Finalizing narrative");

        // Get narrative
        let partial = self
            .narrative_registry()
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
        self.narrative_registry().remove(&narrative_id);

        debug!(narrative_id, success, "Narrative finalized");

        Ok(Json(FinalizeNarrativeResult::new(
            success,
            toml,
            validation_errors,
        )))
    }

    /// Get the current state of a narrative session.
    #[instrument(skip(self, params), fields(narrative_id = params.narrative_id(), format = ?params.format()))]
    pub async fn get_narrative_state(
        &self,
        Parameters(params): Parameters<GetNarrativeStateParams>,
    ) -> Result<Json<GetNarrativeStateResult>, rmcp::ErrorData> {
        let narrative_id = params.narrative_id().clone();
        let format = params.format().clone();

        debug!(narrative_id, ?format, "Getting narrative state");

        // Get narrative
        let partial = self
            .narrative_registry()
            .get(&narrative_id)
            .map_err(|e| to_mcp_error(e, "Narrative not found"))?;

        // Calculate state
        let acts_count = partial.acts().len();
        let acts: Vec<String> = partial.act_order().clone();
        let has_carousel = partial.carousel().is_some()
            || partial.acts().values().any(|act| act.carousel().is_some());

        let metadata_complete = partial.name().is_some()
            && partial.description().is_some()
            && partial.model().is_some();
        let acts_complete = !partial.acts().is_empty()
            && partial.acts().values().all(|act| !act.prompt().is_empty());
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

        let state = NarrativeStateSummary::new(
            partial.name().clone(),
            acts_count,
            acts,
            format!("{}%", completeness_score),
            has_carousel,
        );

        Ok(Json(GetNarrativeStateResult::new(
            narrative_id,
            state,
            toml,
        )))
    }

    /// Validate a narrative session and report issues.
    #[instrument(skip(self, params), fields(narrative_id = params.narrative_id(), strict = params.strict()))]
    pub async fn validate_narrative_session(
        &self,
        Parameters(params): Parameters<ValidateNarrativeSessionParams>,
    ) -> Result<Json<ValidateNarrativeSessionResult>, rmcp::ErrorData> {
        let narrative_id = params.narrative_id().clone();
        let strict = *params.strict();

        debug!(narrative_id, strict, "Validating narrative session");

        // Get narrative
        let partial = self
            .narrative_registry()
            .get(&narrative_id)
            .map_err(|e| to_mcp_error(e, "Narrative not found"))?;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate name
        if partial.name().is_none() || partial.name().as_ref().is_some_and(|n| n.is_empty()) {
            errors.push(ValidationIssue::new(
                ValidationSeverity::Critical,
                "name".to_string(),
                "Narrative name is required".to_string(),
                "Provide a unique name for this narrative".to_string(),
                false,
            ));
        }

        // Validate description
        if partial.description().is_none()
            || partial.description().as_ref().is_some_and(|d| d.is_empty())
        {
            errors.push(ValidationIssue::new(
                ValidationSeverity::High,
                "description".to_string(),
                "Narrative description is missing".to_string(),
                "Add a description explaining what this narrative does".to_string(),
                false,
            ));
        }

        // Validate model
        if partial.model().is_none() {
            warnings.push(ValidationIssue::new(
                ValidationSeverity::Medium,
                "model".to_string(),
                "Default model not specified".to_string(),
                "Set a default model (e.g., 'gemini-2.0-flash-exp')".to_string(),
                true,
            ));
        }

        // Validate acts
        if partial.acts().is_empty() {
            errors.push(ValidationIssue::new(
                ValidationSeverity::Critical,
                "acts".to_string(),
                "Narrative has no acts".to_string(),
                "Add at least one act to the narrative".to_string(),
                false,
            ));
        } else {
            for (act_name, act) in partial.acts() {
                if act.prompt().is_empty() {
                    errors.push(ValidationIssue::new(
                        ValidationSeverity::High,
                        format!("acts.{}.prompt", act_name),
                        format!("Act '{}' has empty prompt", act_name),
                        "Provide a prompt for this act".to_string(),
                        false,
                    ));
                }

                if strict && act.model().is_none() && partial.model().is_none() {
                    warnings.push(ValidationIssue::new(
                        ValidationSeverity::Low,
                        format!("acts.{}.model", act_name),
                        format!("Act '{}' has no model specified", act_name),
                        "Set model for act or narrative default".to_string(),
                        true,
                    ));
                }
            }
        }

        // Calculate completeness
        let metadata_complete = partial.name().is_some()
            && partial.description().is_some()
            && partial.model().is_some();
        let acts_complete = !partial.acts().is_empty()
            && partial.acts().values().all(|act| !act.prompt().is_empty());

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
            .filter(|issue| *issue.auto_fixable())
            .count();

        let is_valid = errors.is_empty();

        debug!(
            narrative_id,
            is_valid,
            errors = errors.len(),
            warnings = warnings.len(),
            "Validation complete"
        );

        Ok(Json(ValidateNarrativeSessionResult::new(
            narrative_id,
            is_valid,
            errors,
            warnings,
            format!("{}%", completeness_score),
            auto_fixable_count,
        )))
    }

    /// Apply automatic fixes to a narrative session based on validation results.
    #[instrument(skip(self, params), fields(narrative_id = params.narrative_id(), fix_count = params.fix_types().len(), confirm = params.confirm()))]
    pub async fn apply_validation_fixes(
        &self,
        Parameters(params): Parameters<ApplyValidationFixesParams>,
    ) -> Result<Json<ApplyValidationFixesResult>, rmcp::ErrorData> {
        let narrative_id = params.narrative_id().clone();
        let fix_types = params.fix_types().clone();
        let confirm = *params.confirm();

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
        let mut partial = self.narrative_registry().get(&narrative_id).map_err(|e| {
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
        self.narrative_registry().add(partial);

        // Count remaining errors by validating
        let remaining_errors = {
            let partial = self
                .narrative_registry()
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

        Ok(Json(ApplyValidationFixesResult::new(
            true,
            fixes_applied,
            remaining_errors,
        )))
    }
}

// =============================================================================
// MCP Tool Wrapper Parameter DTOs
// =============================================================================

/// Parameters for loading a narrative from a file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NarrativeFromFileParams {
    /// Path to the narrative TOML file
    pub path: String,
}

/// Parameters for loading a narrative from a file with database support.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "database")]
pub struct NarrativeFromFileWithDbParams {
    /// Path to the narrative TOML file
    pub path: String,
    /// Database connection string
    pub database_url: String,
}

/// Parameters for creating a StateManager.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateManagerNewParams {
    /// Directory where state files will be stored
    pub state_dir: String,
}

/// Parameters for loading a multi-narrative from a file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultiNarrativeFromFileParams {
    /// Path to the multi-narrative TOML file
    pub path: String,
    /// Name of the narrative within the file
    pub narrative_name: String,
}

/// Parameters for loading a multi-narrative from a file with database support.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "database")]
pub struct MultiNarrativeFromFileWithDbParams {
    /// Path to the multi-narrative TOML file
    pub path: String,
    /// Name of the narrative within the file
    pub narrative_name: String,
    /// Database connection string
    pub database_url: String,
}

/// Parameters for loading a narrative from a TOML string.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NarrativeFromTomlStrParams {
    /// TOML content to parse
    pub content: String,
    /// Optional source description for error messages
    pub source: Option<String>,
}

/// Parameters for assembling narrative act prompts with database content.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "database")]
pub struct AssembleNarrativeActPromptsParams {
    /// The narrative to process (will be modified in place)
    pub narrative: Narrative,
    /// Database connection string
    pub database_url: String,
}

// =============================================================================
// MCP Tool Wrapper Implementations
// =============================================================================

impl BotticelliServer {
    // =========================================================================
    // MCP Tool Wrappers for botticelli_narrative
    //
    // Wrappers for narrative functions that cannot be directly tooled:
    // - Generic path parameters (AsRef<Path>)
    // - Borrowed string parameters (&str)
    // - Async methods with &self/&mut self
    // - Trait implementations
    // - Generic trait bounds
    // =========================================================================

    /// Load a narrative from a TOML file.
    ///
    /// Wrapper for `Narrative::from_file<P: AsRef<Path>>`.
    #[tool]
    #[tracing::instrument(skip(self, params), fields(path = %params.path))]
    pub fn narrative_from_file(
        &self,
        params: NarrativeFromFileParams,
    ) -> Result<Narrative, botticelli_error::BotticelliError> {
    use std::path::PathBuf;
    let path = PathBuf::from(params.path);
    tracing::debug!(path = %path.display(), "Loading narrative from file");
    Narrative::from_file(&path).map_err(Into::into)
}

    /// Load a narrative from a TOML file with database schema reflection.
    ///
    /// Wrapper for `Narrative::from_file_with_db<P: AsRef<Path>>`.
    #[tool]
    #[cfg(feature = "database")]
    #[tracing::instrument(skip(self, params), fields(path = %params.path))]
    pub fn narrative_from_file_with_db(
        &self,
        params: NarrativeFromFileWithDbParams,
    ) -> Result<Narrative, botticelli_error::BotticelliError> {
    use diesel::prelude::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use std::path::PathBuf;

    let path = PathBuf::from(params.path);
    tracing::debug!(path = %path.display(), "Loading narrative from file with database");

    // Create connection pool
    let manager = ConnectionManager::<PgConnection>::new(&params.database_url);
    let pool = Pool::builder().build(manager).map_err(|e| {
        botticelli_error::BackendError::new(format!("Failed to create connection pool: {}", e))
    })?;

    let mut conn = pool.get().map_err(|e| {
        botticelli_error::BackendError::new(format!("Failed to get connection: {}", e))
    })?;

    Narrative::from_file_with_db(&path, &mut conn).map_err(Into::into)
}

/// Create a new state manager.
///
/// Wrapper for `StateManager::new<P: AsRef<Path>>`.
#[tool]
#[tracing::instrument(skip(self, params), fields(state_dir = %params.state_dir))]
pub fn state_manager_new(
    &self,
    params: StateManagerNewParams,
) -> Result<StateManager, botticelli_error::BotticelliError> {
    use std::path::PathBuf;
    let path = PathBuf::from(params.state_dir);
    tracing::debug!(path = %path.display(), "Creating state manager");
    StateManager::new(&path)
}

/// Load all narratives from a TOML file.
///
/// Wrapper for `MultiNarrative::from_file<P: AsRef<Path>>`.
#[tool]
#[tracing::instrument(skip(self, params), fields(path = %params.path, narrative_name = %params.narrative_name))]
pub fn multi_narrative_from_file(
    &self,
    params: MultiNarrativeFromFileParams,
) -> Result<MultiNarrative, botticelli_error::BotticelliError> {
    use std::path::PathBuf;
    let path = PathBuf::from(params.path);
    tracing::debug!(
        path = %path.display(),
        narrative_name = %params.narrative_name,
        "Loading multi-narrative from file"
    );
    MultiNarrative::from_file(&path, &params.narrative_name).map_err(Into::into)
}

/// Load all narratives from a TOML file with database support.
///
/// Wrapper for `MultiNarrative::from_file_with_db<P: AsRef<Path>>`.
#[tool]
#[cfg(feature = "database")]
#[tracing::instrument(skip(self, params), fields(path = %params.path, narrative_name = %params.narrative_name))]
pub fn multi_narrative_from_file_with_db(
    &self,
    params: MultiNarrativeFromFileWithDbParams,
) -> Result<MultiNarrative, botticelli_error::BotticelliError> {
    use diesel::prelude::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use std::path::PathBuf;

    let path = PathBuf::from(params.path);
    tracing::debug!(
        path = %path.display(),
        narrative_name = %params.narrative_name,
        "Loading multi-narrative from file with database"
    );

    // Create connection pool
    let manager = ConnectionManager::<PgConnection>::new(&params.database_url);
    let pool = Pool::builder().build(manager).map_err(|e| {
        botticelli_error::BackendError::new(format!("Failed to create connection pool: {}", e))
    })?;

    let mut conn = pool.get().map_err(|e| {
        botticelli_error::BackendError::new(format!("Failed to get connection: {}", e))
    })?;

    MultiNarrative::from_file_with_db(&path, &params.narrative_name, &mut conn).map_err(Into::into)
}

// -----------------------------------------------------------------------------
// Category 2: Borrowed String Functions
// -----------------------------------------------------------------------------

/// Parse a narrative from a TOML string.
///
/// Wrapper for `Narrative::from_toml_str(&str, Option<&str>)`.
#[tool]
#[tracing::instrument(skip(self, params), fields(content_len = params.content.len(), source = ?params.source))]
pub fn narrative_from_toml_str(
    &self,
    params: NarrativeFromTomlStrParams,
) -> Result<Narrative, botticelli_error::BotticelliError> {
    tracing::debug!(
        content_len = params.content.len(),
        source = ?params.source,
        "Parsing narrative from TOML string"
    );
    Narrative::from_toml_str(&params.content, params.source.as_deref()).map_err(Into::into)
}

// -----------------------------------------------------------------------------
// Category 3: Database Connection Functions
// -----------------------------------------------------------------------------

/// Assemble act prompts from database templates.
///
/// Wrapper for `Narrative::assemble_act_prompts(&mut PgConnection)`.
#[tool]
#[cfg(feature = "database")]
#[tracing::instrument(skip(self, params), fields(narrative_name = params.narrative.metadata().name()))]
pub fn assemble_narrative_act_prompts(
    &self,
    mut params: AssembleNarrativeActPromptsParams,
) -> Result<Narrative, botticelli_error::BotticelliError> {
    use diesel::prelude::*;
    use diesel::r2d2::{ConnectionManager, Pool};

    tracing::debug!(
        narrative_name = params.narrative.metadata().name(),
        "Assembling narrative act prompts"
    );

    // Create connection pool
    let manager = ConnectionManager::<PgConnection>::new(&params.database_url);
    let pool = Pool::builder().build(manager).map_err(|e| {
        botticelli_error::BackendError::new(format!("Failed to create connection pool: {}", e))
    })?;

    let mut conn = pool.get().map_err(|e| {
        botticelli_error::BackendError::new(format!("Failed to get connection: {}", e))
    })?;

    params.narrative.assemble_act_prompts(&mut conn).map_err(Into::into)?;
    Ok(params.narrative)
}
}
