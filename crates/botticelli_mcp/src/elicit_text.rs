//! Elicit text tool types for free-form text input.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting free-form text input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct ElicitTextParams {
    /// The prompt to display to the user
    prompt: String,
}

impl ElicitTextParams {
    /// Create new elicit text parameters.
    #[tracing::instrument(skip(prompt), fields(prompt_len = prompt.len()))]
    pub fn new(prompt: String) -> Self {
        Self { prompt }
    }
}

/// Result from text elicitation.
///
/// Returns the user's text input directly as a string value.
/// The elicitation crate expects the raw text, not wrapped in an object.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ElicitTextResult {
    /// The text value entered by the user
    value: String,
}

impl ElicitTextResult {
    /// Create a new elicit text result.
    ///
    /// # Arguments
    ///
    /// * `value` - The text value entered by the user
    ///
    /// # Returns
    ///
    /// A new `ElicitTextResult`.
    #[tracing::instrument(skip(value), fields(value_len = value.len()))]
    pub fn new(value: String) -> Self {
        Self { value }
    }
}
