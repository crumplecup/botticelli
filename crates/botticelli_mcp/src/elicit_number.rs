//! Elicit number tool types for numeric input with range constraints.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting numeric input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ElicitNumberParams {
    /// The prompt to display to the user
    pub prompt: String,

    /// Minimum acceptable value (inclusive)
    pub min: i64,

    /// Maximum acceptable value (inclusive)
    pub max: i64,
}

/// Result from numeric elicitation.
///
/// Returns the user's numeric input within the specified range.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ElicitNumberResult {
    /// The numeric value from the user's input
    pub value: i64,
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
    pub fn new(value: i64) -> Self {
        Self { value }
    }
}
