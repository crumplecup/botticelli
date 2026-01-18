//! Elicit number tool types for numeric input with range constraints.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting numeric input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, derive_new::new)]
pub struct ElicitNumberParams {
    /// The prompt to display to the user
    prompt: String,

    /// Minimum acceptable value (inclusive)
    min: i64,

    /// Maximum acceptable value (inclusive)
    max: i64,
}

/// Result from numeric elicitation.
///
/// Returns the user's numeric input within the specified range.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters, derive_new::new)]
pub struct ElicitNumberResult {
    /// The numeric value from the user's input
    value: i64,
}
