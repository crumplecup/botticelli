//! Validation error types for TOML narrative validation.

/// Result of validating a narrative TOML file.
#[derive(Debug, Clone, Default, derive_getters::Getters)]
pub struct ValidationResult {
    /// Validation errors (must be fixed)
    errors: Vec<ValidationError>,
    /// Validation warnings (should be reviewed)
    warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Creates a new validation result with no errors or warnings.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if validation passed (no errors).
    #[tracing::instrument(skip(self))]
    pub fn is_valid(&self) -> bool {
        let is_valid = self.errors().is_empty();
        tracing::debug!(
            is_valid,
            error_count = self.errors().len(),
            warning_count = self.warnings().len(),
            "Validation result checked"
        );
        is_valid
    }

    /// Adds an error to the result.
    #[tracing::instrument(skip(self, error), fields(error_kind = ?error.kind()))]
    pub fn add_error(&mut self, error: ValidationError) {
        tracing::debug!(
            message = %error.message(),
            has_suggestion = error.suggestion().is_some(),
            "Validation error added"
        );
        self.errors.push(error);
    }

    /// Adds a warning to the result.
    #[tracing::instrument(skip(self, warning), fields(warning_kind = ?warning.kind()))]
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        tracing::debug!(message = %warning.message(), "Validation warning added");
        self.warnings.push(warning);
    }

    /// Formats errors as a human-readable string.
    #[tracing::instrument(skip(self), fields(error_count = self.errors().len()))]
    pub fn format_errors(&self) -> String {
        let mut output = String::new();

        for (i, error) in self.errors().iter().enumerate() {
            if i > 0 {
                output.push_str("\n\n");
            }
            output.push_str(&format!("Error {}: {}", i + 1, error.message()));

            if let Some(suggestion) = error.suggestion() {
                output.push_str(&format!("\n\n  Suggestion: {}", suggestion));
            }
        }

        tracing::debug!(output_len = output.len(), "Formatted errors");
        output
    }

    /// Formats warnings as a human-readable string.
    #[tracing::instrument(skip(self), fields(warning_count = self.warnings().len()))]
    pub fn format_warnings(&self) -> String {
        let mut output = String::new();

        for (i, warning) in self.warnings().iter().enumerate() {
            if i > 0 {
                output.push_str("\n\n");
            }
            output.push_str(&format!("Warning {}: {}", i + 1, warning.message()));
        }

        tracing::debug!(output_len = output.len(), "Formatted warnings");
        output
    }
}

/// A validation error with location and fix suggestion.
#[derive(Debug, Clone, derive_getters::Getters, derive_new::new)]
pub struct ValidationError {
    /// Type of validation error
    kind: ValidationErrorKind,
    /// Location in the TOML file (if available)
    location: Option<ValidationLocation>,
    /// Human-readable error message
    message: String,
    /// Suggestion on how to fix the error
    suggestion: Option<String>,
}

/// A validation warning that should be reviewed.
#[derive(Debug, Clone, derive_getters::Getters, derive_new::new)]
pub struct ValidationWarning {
    /// Type of validation warning
    kind: ValidationWarningKind,
    /// Location in the TOML file (if available)
    location: Option<ValidationLocation>,
    /// Human-readable warning message
    message: String,
}

/// Location information for validation messages.
#[derive(Debug, Clone, derive_getters::Getters, derive_new::new)]
pub struct ValidationLocation {
    /// Line number (1-indexed)
    line: usize,
    /// Column number (1-indexed)
    column: usize,
    /// Section name (e.g., "acts.fetch_data")
    section: Option<String>,
}

/// Types of validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationErrorKind {
    /// Invalid TOML syntax pattern
    InvalidSyntax,
    /// Missing required section
    MissingSection,
    /// Undefined resource reference
    UndefinedReference,
    /// Empty table of contents
    EmptyToc,
    /// Act referenced in toc but not defined
    MissingAct,
    /// Act has no inputs
    EmptyPrompt,
    /// File not found
    FileNotFound,
    /// Circular dependency detected
    CircularDependency,
}

/// Types of validation warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationWarningKind {
    /// Unknown model name (possible typo)
    UnknownModel,
    /// Defined resource never used
    UnusedResource,
    /// Direct table reference without [tables] definition
    DirectTableReference,
    /// Large media file (may impact performance)
    LargeMediaFile,
}
