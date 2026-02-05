//! Model name validation and fuzzy matching.

use botticelli_error::{
    ValidationLocation, ValidationResult, ValidationWarning, ValidationWarningKind,
};
use rmcp::tool;
use tracing::instrument;

/// Known model names for validation.
const KNOWN_MODELS: &[&str] = &[
    // Gemini models
    "gemini-2.0-flash-exp",
    "gemini-1.5-flash",
    "gemini-1.5-flash-8b",
    "gemini-1.5-pro",
    "gemini-exp-1206",
    // OpenAI models
    "gpt-4",
    "gpt-4-turbo",
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-3.5-turbo",
    // Anthropic models
    "claude-3-5-sonnet-20241022",
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    // Groq models
    "llama-3.3-70b-versatile",
    "llama-3.1-70b-versatile",
    "llama-3.1-8b-instant",
    "mixtral-8x7b-32768",
    // HuggingFace (common examples)
    "meta-llama/Meta-Llama-3-8B-Instruct",
    "mistralai/Mistral-7B-Instruct-v0.2",
    // Ollama (common models)
    "llama3.2",
    "llama3.1",
    "mistral",
    "phi3",
];

/// Unit struct providing model validation methods.
///
/// Groups model-related validation functions under a clean namespace.
#[derive(Debug, Clone, Copy)]
pub struct ModelValidator;

impl ModelValidator {
    /// Validates a model name against known models.
    #[instrument(skip(section, result), fields(section = %section_name))]
    #[tool]
    pub fn validate_name(
        section: &toml::map::Map<String, toml::Value>,
        section_name: &str,
        result: &mut ValidationResult,
    ) {
        if let Some(model) = section.get("model").and_then(|v| v.as_str())
            && !KNOWN_MODELS.contains(&model)
        {
            tracing::debug!(model = %model, "Unknown model found");
            // Try to find a close match for suggestions
            let suggestion = Self::find_closest(model);

            result.add_warning(ValidationWarning::new(
                    ValidationWarningKind::UnknownModel,
                    Some(ValidationLocation::new(
                        0,
                        0,
                        Some(section_name.to_string()),
                    )),
                    if let Some(closest) = suggestion {
                        format!("Unknown model '{}'. Did you mean '{}'?", model, closest)
                    } else {
                        format!(
                            "Unknown model '{}'. This may be a typo or a newer model not in the validator's list.",
                            model
                        )
                    },
                ));
        }
    }

    /// Finds the closest matching model name using simple string distance.
    #[instrument(fields(model = %model, match_found = tracing::field::Empty))]
    fn find_closest(model: &str) -> Option<&'static str> {
        let model_lower = model.to_lowercase();

        // First try exact substring match
        for known in KNOWN_MODELS {
            if known.to_lowercase().contains(&model_lower)
                || model_lower.contains(&known.to_lowercase())
            {
                tracing::Span::current().record("match_found", true);
                tracing::debug!(closest = %known, "Found substring match");
                return Some(known);
            }
        }

        // Try Levenshtein distance for close matches
        let mut best_match: Option<(&str, usize)> = None;
        for known in KNOWN_MODELS {
            let distance = levenshtein_distance(&model_lower, &known.to_lowercase());
            if distance <= 3 {
                // Allow up to 3 character differences
                if let Some((_, best_dist)) = best_match {
                    if distance < best_dist {
                        best_match = Some((known, distance));
                    }
                } else {
                    best_match = Some((known, distance));
                }
            }
        }

        if let Some((model, dist)) = best_match {
            tracing::Span::current().record("match_found", true);
            tracing::debug!(closest = %model, distance = dist, "Found Levenshtein match");
        } else {
            tracing::Span::current().record("match_found", false);
            tracing::debug!("No close match found");
        }

        best_match.map(|(model, _)| model)
    }
}

/// Computes the Levenshtein distance between two strings.
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i;
    }
    for (j, val) in matrix[0].iter_mut().enumerate().take(len2 + 1) {
        *val = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };

            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }

    matrix[len1][len2]
}
