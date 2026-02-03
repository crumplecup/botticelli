//! Core validation orchestration.

use rmcp::tool;
use super::{
    analysis::Analyzer, models::ModelValidator, resources::ResourceValidator,
    structure::StructureValidator, syntax::SyntaxValidator,
};
use botticelli_error::{ValidationError, ValidationErrorKind, ValidationResult};
use std::path::Path;
use tracing::instrument;

/// Configuration for validation behavior.
#[derive(Debug, Clone, derive_getters::Getters, derive_setters::Setters)]
#[setters(prefix = "with_")]
pub struct ValidationConfig {
    /// Check that nested narrative files exist
    validate_nested_narratives: bool,
    /// Check that media files exist
    validate_media_files: bool,
    /// Warn on unknown model names
    warn_unknown_models: bool,
    /// Warn on unused resources
    warn_unused_resources: bool,
    /// Base directory for relative path resolution
    base_dir: Option<std::path::PathBuf>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            validate_nested_narratives: true,
            validate_media_files: true,
            warn_unknown_models: true,
            warn_unused_resources: true,
            base_dir: None,
        }
    }
}

/// Core validation orchestrator.
pub struct Validator;

impl Validator {
    /// Validates a narrative TOML string.
    ///
    /// # Arguments
    ///
    /// * `toml` - The TOML content to validate
    ///
    /// # Returns
    ///
    /// A `ValidationResult` containing any errors or warnings found.
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_narrative::validator::Validator;
    ///
    /// let toml = r#"
    ///     [narrative]
    ///     name = "test"
    ///     description = "Test narrative"
    ///     
    ///     [toc]
    ///     order = ["act1"]
    ///     
    ///     [acts]
    ///     act1 = "Hello world"
    /// "#;
    ///
    /// let result = Validator::validate_toml(toml);
    /// assert!(result.is_valid());
    /// ```
    #[instrument(skip(toml), fields(toml_len = toml.len()))]
    #[tool]
    pub fn validate_toml(toml: &str) -> ValidationResult {
        Self::validate_toml_with_config(toml, &ValidationConfig::default())
    }

    /// Validates a narrative TOML string with custom configuration.
    #[instrument(skip(toml, config), fields(toml_len = toml.len()))]
    #[tool]
    pub fn validate_toml_with_config(toml: &str, config: &ValidationConfig) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Phase 1: Parse TOML
        let parsed = match toml::from_str::<toml::Value>(toml) {
            Ok(value) => value,
            Err(e) => {
                result.add_error(ValidationError::new(
                    ValidationErrorKind::InvalidSyntax,
                    None,
                    format!("Failed to parse TOML: {}", e),
                    Some("Check for syntax errors like missing quotes, unmatched brackets, or invalid escape sequences.".to_string()),
                ));
                return result;
            }
        };

        // Phase 2: Check for common syntax patterns
        SyntaxValidator::detect_patterns(&parsed, &mut result);

        // Phase 3: Validate structure (sections, references, etc.)
        Self::validate_structure(&parsed, config, &mut result);

        result
    }

    /// Validates a narrative TOML file.
    #[instrument(skip(path), fields(path = %path.as_ref().display()))]
    #[tool]
    pub fn validate_file(path: impl AsRef<Path>) -> ValidationResult {
        Self::validate_file_with_config(path, &ValidationConfig::default())
    }

    /// Validates a narrative TOML file with custom configuration.
    #[instrument(skip(path, config), fields(path = %path.as_ref().display()))]
    #[tool]
    pub fn validate_file_with_config(
        path: impl AsRef<Path>,
        config: &ValidationConfig,
    ) -> ValidationResult {
        let path = path.as_ref();
        let mut result = ValidationResult::new();

        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                result.add_error(ValidationError::new(
                    ValidationErrorKind::FileNotFound,
                    None,
                    format!("Failed to read file '{}': {}", path.display(), e),
                    Some("Check that the file exists and is readable.".to_string()),
                ));
                return result;
            }
        };

        Self::validate_toml_with_config(&content, config)
    }

    /// Validates narrative structure (sections, references, etc.).
    #[instrument(skip(parsed, config, result), fields(
        has_narrative = tracing::field::Empty,
        has_narratives = tracing::field::Empty,
        resource_count = tracing::field::Empty
    ))]
    fn validate_structure(
        parsed: &toml::Value,
        config: &ValidationConfig,
        result: &mut ValidationResult,
    ) {
        let table = match parsed.as_table() {
            Some(t) => t,
            None => {
                tracing::error!("TOML root is not a table");
                result.add_error(ValidationError::new(
                    ValidationErrorKind::InvalidSyntax,
                    None,
                    "TOML root must be a table".to_string(),
                    None,
                ));
                return;
            }
        };

        // Check for [narrative] section
        let has_narrative = table.contains_key("narrative");
        let has_narratives = table.contains_key("narratives");

        tracing::Span::current().record("has_narrative", has_narrative);
        tracing::Span::current().record("has_narratives", has_narratives);
        tracing::debug!(has_narrative, has_narratives, "Detected narrative sections");

        if !has_narrative && !has_narratives {
            tracing::error!("No narrative section found");
            result.add_error(ValidationError::new(
            ValidationErrorKind::MissingSection,
            None,
            "Missing [narrative] or [narratives] section".to_string(),
            Some("Add a [narrative] section:\n\n[narrative]\nname = \"my_narrative\"\ndescription = \"...\"".to_string()),
        ));
            return;
        }

        // Validate model names if enabled
        if *config.warn_unknown_models() {
            if has_narrative
                && let Some(narrative) = table.get("narrative").and_then(|v| v.as_table())
            {
                ModelValidator::validate_name(narrative, "narrative", result);
            }
            if has_narratives
                && let Some(narratives) = table.get("narratives").and_then(|v| v.as_table())
            {
                for (name, narrative_value) in narratives {
                    if let Some(narrative_table) = narrative_value.as_table() {
                        ModelValidator::validate_name(
                            narrative_table,
                            &format!("narratives.{}", name),
                            result,
                        );
                    }
                }
            }
            // Check acts for model overrides
            if let Some(acts) = table.get("acts").and_then(|v| v.as_table()) {
                for (act_name, act_value) in acts {
                    if let Some(act_table) = act_value.as_table() {
                        ModelValidator::validate_name(
                            act_table,
                            &format!("acts.{}", act_name),
                            result,
                        );
                    }
                }
            }
        }

        // Collect resources for reference validation and unused detection
        let resources = ResourceValidator::collect(table);
        let resource_count =
            resources.bots().len() + resources.tables().len() + resources.media().len();
        tracing::Span::current().record("resource_count", resource_count);
        tracing::debug!(
            bots = resources.bots().len(),
            tables = resources.tables().len(),
            media = resources.media().len(),
            "Collected resources"
        );

        // For single narrative files, validate toc and acts
        if has_narrative {
            StructureValidator::validate_single(table, &resources, result);
        }

        // For multi-narrative files, each narrative has its own toc
        if has_narratives {
            StructureValidator::validate_multi(table, &resources, result);
        }

        // Check for unused resources if enabled
        if *config.warn_unused_resources() {
            Analyzer::check_unused_resources(&resources, result);
        }

        // Check for circular dependencies in narrative references
        Analyzer::check_circular_dependencies(table, result);
    }
}
