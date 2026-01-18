//! Elicit select tool types for choosing from a list of options.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting a selection from options.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct ElicitSelectParams {
    /// The prompt to display to the user
    prompt: String,

    /// Array of valid options to choose from
    options: Vec<String>,
}

impl ElicitSelectParams {
    /// Create new elicit select parameters.
    #[tracing::instrument(skip(prompt, options), fields(prompt_len = prompt.len(), options_count = options.len()))]
    pub fn new(prompt: String, options: Vec<String>) -> Self {
        Self { prompt, options }
    }
}

/// Result from selection elicitation.
///
/// Returns the user's selected option from the provided list.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ElicitSelectResult {
    /// The selected option value
    value: String,
}

impl ElicitSelectResult {
    /// Create a new elicit select result.
    ///
    /// # Arguments
    ///
    /// * `value` - The selected option value
    ///
    /// # Returns
    ///
    /// A new `ElicitSelectResult`.
    #[tracing::instrument(skip(value), fields(value_len = value.len()))]
    pub fn new(value: String) -> Self {
        Self { value }
    }
}
