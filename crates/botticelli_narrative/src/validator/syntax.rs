//! TOML syntax pattern validation.

use botticelli_error::{ValidationError, ValidationErrorKind, ValidationResult};
use tracing::instrument;

/// Unit struct providing syntax validation methods.
///
/// Groups syntax-related validation functions under a clean namespace.
#[derive(Debug, Clone, Copy)]
pub struct SyntaxValidator;

impl SyntaxValidator {
    /// Detects common TOML syntax patterns that cause errors.
    #[instrument(skip(parsed, result))]
    pub fn detect_patterns(parsed: &toml::Value, result: &mut ValidationResult) {
        if let Some(table) = parsed.as_table() {
            // Check for [[acts]] (array of tables instead of table of tables)
            if let Some(acts_value) = table.get("acts")
                && acts_value.is_array()
            {
                tracing::debug!("Found invalid [[acts]] array syntax");
                result.add_error(ValidationError::new(
                    ValidationErrorKind::InvalidSyntax,
                    None,
                    "Found [[acts]] but acts should be a table of tables, not an array"
                        .to_string(),
                    Some(
                        "Use one of these formats:\n\n\
                        1. Inline table syntax (recommended for simple prompts):\n\
                           [acts]\n\
                           fetch = \"Get data\"\n\n\
                        2. Table syntax (for acts with configuration):\n\
                           [acts.fetch]\n\
                           prompt = \"Get data\"\n\
                           model = \"gemini-2.0-flash-exp\""
                            .to_string(),
                    ),
                ));
            }

            // Check for [[narrative]] (multiple narrative sections)
            if let Some(narrative_value) = table.get("narrative")
                && narrative_value.is_array()
            {
                tracing::debug!("Found invalid [[narrative]] array syntax");
                result.add_error(ValidationError::new(
                    ValidationErrorKind::InvalidSyntax,
                    None,
                    "Found [[narrative]] but only a single [narrative] section is allowed".to_string(),
                    Some("Use a single [narrative] section:\n\n[narrative]\nname = \"my_narrative\"\ndescription = \"...\"".to_string()),
                ));
            }
        }
    }
}
