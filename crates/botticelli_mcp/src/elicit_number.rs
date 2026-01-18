//! Elicit number tool types for numeric input with range constraints.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting numeric input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct ElicitNumberParams {
    /// The prompt to display to the user
    prompt: String,

    /// Minimum acceptable value (inclusive)
    min: i64,

    /// Maximum acceptable value (inclusive)
    max: i64,
}

impl ElicitNumberParams {
    /// Create new elicit number parameters.
    #[tracing::instrument(skip(prompt), fields(prompt_len = prompt.len()))]
    pub fn new(prompt: String, min: i64, max: i64) -> Self {
        Self { prompt, min, max }
    }
}

/// Result from numeric elicitation.
///
/// Returns the user's numeric input within the specified range.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ElicitNumberResult {
    /// The numeric value from the user's input
    value: i64,
}

impl ElicitNumberResult {
    /// Create a new elicit number result.
    ///
    /// # Arguments
    ///
    /// * `value` - The numeric input value
    ///
    /// # Returns
    ///
    /// A new `ElicitNumberResult`.
    #[tracing::instrument]
    pub fn new(value: i64) -> Self {
        Self { value }
    }
}
