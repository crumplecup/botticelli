//! Validate narrative tool types.
//!
//! Validates narrative TOML files for syntax, structure, and references.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Parameters for validating a narrative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, derive_builder::Builder, Default)]
#[builder(setter(into), default)]
pub struct ValidateNarrativeParams {
    /// TOML content to validate (either this or file_path must be provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    content: Option<String>,

    /// Path to TOML file to validate (either this or content must be provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    file_path: Option<String>,

    /// Check that media and nested narrative files exist.
    #[serde(default)]
    #[builder(default)]
    validate_files: bool,

    /// Warn on unknown model names.
    #[serde(default = "default_validate_models")]
    #[builder(default = "default_validate_models()")]
    validate_models: bool,

    /// Warn about unused resources (bots, tables, media).
    #[serde(default = "default_warn_unused")]
    #[builder(default = "default_warn_unused()")]
    warn_unused: bool,

    /// Treat warnings as errors.
    #[serde(default)]
    #[builder(default)]
    strict: bool,
}

impl ValidateNarrativeParams {
    /// Create a builder for validate narrative parameters.
    #[instrument]
    pub fn builder() -> ValidateNarrativeParamsBuilder {
        ValidateNarrativeParamsBuilder::default()
    }
}

#[tracing::instrument]
fn default_validate_models() -> bool {
    true
}

#[tracing::instrument]
fn default_warn_unused() -> bool {
    true
}

/// Location information for a validation issue.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ValidationLocation {
    /// Line number in the TOML file.
    line: usize,

    /// Column number in the TOML file.
    column: usize,

    /// Section where the issue occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    section: Option<String>,
}

impl ValidationLocation {
    /// Create new validation location.
    #[instrument]
    pub fn new(line: usize, column: usize, section: Option<String>) -> Self {
        Self {
            line,
            column,
            section,
        }
    }
}

/// A validation error with location and suggestion.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ValidationError {
    /// Kind of error (e.g., "SyntaxError", "ReferenceError").
    kind: String,

    /// Error message.
    message: String,

    /// Optional suggestion for fixing the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,

    /// Optional location information.
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<ValidationLocation>,
}

impl ValidationError {
    /// Create new validation error.
    #[instrument(skip(location))]
    pub fn new(
        kind: String,
        message: String,
        suggestion: Option<String>,
        location: Option<ValidationLocation>,
    ) -> Self {
        Self {
            kind,
            message,
            suggestion,
            location,
        }
    }
}

/// A validation warning with location.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ValidationWarning {
    /// Kind of warning (e.g., "UnknownModel", "UnusedResource").
    kind: String,

    /// Warning message.
    message: String,

    /// Optional location information.
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<ValidationLocation>,
}

impl ValidationWarning {
    /// Create new validation warning.
    #[instrument(skip(location))]
    pub fn new(kind: String, message: String, location: Option<ValidationLocation>) -> Self {
        Self {
            kind,
            message,
            location,
        }
    }
}

/// Result from validating a narrative.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ValidateNarrativeResult {
    /// Whether the narrative is valid (no errors, or no warnings if strict).
    valid: bool,

    /// List of validation errors.
    errors: Vec<ValidationError>,

    /// List of validation warnings.
    warnings: Vec<ValidationWarning>,

    /// Summary message.
    summary: String,
}

impl ValidateNarrativeResult {
    /// Create a new validation result.
    #[instrument(skip(errors, warnings))]
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
