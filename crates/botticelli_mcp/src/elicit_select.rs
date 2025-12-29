//! Elicit select tool types for choosing from a list of options.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting a selection from options.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ElicitSelectParams {
    /// The prompt to display to the user
    pub prompt: String,

    /// Array of valid options to choose from
    pub options: Vec<String>,
}

/// Result from selection elicitation.
///
/// Returns the user's selected option from the provided list.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ElicitSelectResult {
    /// The selected option value
    pub value: String,
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
    pub fn new(value: String) -> Self {
        Self { value }
    }
}
