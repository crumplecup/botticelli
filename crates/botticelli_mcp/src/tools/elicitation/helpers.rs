use crate::{PartialAct, PartialNarrative};
use botticelli_error::{McpError, McpErrorKind, McpResult};
use tracing::instrument;

/// Analysis results from narrative description.
pub struct DescriptionAnalysis {
    pub suggested_name: String,
    pub detected_acts: Vec<String>,
    pub complexity: String,
    pub recommendations: Vec<String>,
}

/// Helper methods for narrative elicitation.
pub struct ElicitationHelper;

impl ElicitationHelper {
    /// Analyzes a narrative description to extract structure and suggestions.
    #[instrument]
    pub fn analyze_description(description: &str) -> DescriptionAnalysis {
        let suggested_name = Self::extract_suggested_name(description);
        let word_count = description.split_whitespace().count();

        let has_steps = description.to_lowercase().contains("step")
            || description.to_lowercase().contains("then")
            || description.to_lowercase().contains("first");
        let has_data = description.to_lowercase().contains("database")
            || description.to_lowercase().contains("table")
            || description.to_lowercase().contains("query");
        let has_iteration = description.to_lowercase().contains("repeat")
            || description.to_lowercase().contains("loop")
            || description.to_lowercase().contains("iterate")
            || description.to_lowercase().contains("for each");

        let complexity = if word_count < 20 {
            "simple"
        } else if word_count < 50 {
            "moderate"
        } else {
            "complex"
        };

        let mut recommendations = Vec::new();
        if has_steps {
            recommendations.push("Consider using sequential or named acts for the steps".to_string());
        }
        if has_data {
            recommendations.push("Consider adding Table inputs for data sources".to_string());
        }
        if has_iteration {
            recommendations.push("Consider adding a carousel for iteration".to_string());
        }
        if word_count < 10 {
            recommendations.push("Provide more detail about the workflow".to_string());
        }

        let detected_acts = Self::detect_acts_in_description(description);

        DescriptionAnalysis {
            suggested_name,
            detected_acts,
            complexity: complexity.to_string(),
            recommendations,
        }
    }

    /// Extracts a suggested narrative name from description.
    #[instrument]
    pub fn extract_suggested_name(description: &str) -> String {
        description
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join("_")
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }

    /// Detects potential act boundaries in description.
    #[instrument]
    pub fn detect_acts_in_description(description: &str) -> Vec<String> {
        let mut acts = Vec::new();
        let lower = description.to_lowercase();

        if lower.contains("first") || lower.contains("step 1") {
            acts.push("act1".to_string());
        }
        if lower.contains("then") || lower.contains("step 2") {
            acts.push("act2".to_string());
        }
        if lower.contains("finally") || lower.contains("step 3") {
            acts.push("act3".to_string());
        }

        if acts.is_empty() {
            acts.push("act1".to_string());
        }

        acts
    }

    /// Extracts partial acts from description text.
    #[instrument]
    pub fn extract_acts_from_description(description: &str) -> McpResult<Vec<PartialAct>> {
        let detected = Self::detect_acts_in_description(description);
        Ok(detected
            .into_iter()
            .map(|name| PartialAct::new(name, None, None, Vec::new(), None))
            .collect())
    }

    /// Suggests input types based on act prompt.
    #[instrument]
    pub fn suggest_inputs_for_act(prompt: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let lower = prompt.to_lowercase();

        if lower.contains("image") || lower.contains("photo") || lower.contains("picture") {
            suggestions.push("Consider adding Image input".to_string());
        }
        if lower.contains("database") || lower.contains("query") || lower.contains("table") {
            suggestions.push("Consider adding Table input".to_string());
        }
        if lower.contains("document") || lower.contains("file") {
            suggestions.push("Consider adding Document input".to_string());
        }

        suggestions
    }

    /// Validates narrative metadata completeness.
    #[instrument]
    pub fn validate_metadata(partial: &PartialNarrative) -> Vec<String> {
        let mut missing = Vec::new();

        if partial.name.is_none() {
            missing.push("name".to_string());
        }
        if partial.description.is_none() {
            missing.push("description".to_string());
        }

        missing
    }

    /// Generates warnings for narrative metadata quality.
    #[instrument]
    pub fn metadata_warnings(partial: &PartialNarrative) -> Vec<String> {
        let mut warnings = Vec::new();

        if let Some(name) = &partial.name {
            if !Self::is_valid_name(name) {
                warnings.push("Name contains invalid characters".to_string());
            }
        }

        warnings
    }

    #[instrument]
    pub fn validate_complete(partial: &PartialNarrative) -> Vec<String> {
        let mut errors = Vec::new();

        if partial.name.is_none() {
            errors.push("Missing narrative name".to_string());
        }
        if partial.description.is_none() {
            errors.push("Missing narrative description".to_string());
        }
        if partial.acts.is_empty() {
            errors.push("No acts defined".to_string());
        }

        errors
    }

    /// Checks if a narrative name is valid.
    #[instrument]
    pub fn is_valid_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 64
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Creates an error for missing required field.
    #[track_caller]
    pub fn missing_field(field: &str) -> McpError {
        McpError::new(McpErrorKind::InvalidInput(format!("Missing field: {}", field)))
    }

    /// Creates an error for invalid field value.
    #[track_caller]
    pub fn invalid_value(field: &str, reason: &str) -> McpError {
        McpError::new(McpErrorKind::InvalidInput(format!(
            "Invalid value for {}: {}",
            field, reason
        )))
    }

    /// Creates a serialization error.
    #[track_caller]
    pub fn serialization_error(message: &str) -> McpError {
        McpError::new(McpErrorKind::SerializationError(message.to_string()))
    }
}
