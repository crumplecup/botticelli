//! Elicit bool tool types for yes/no confirmation input.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting boolean confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ElicitBoolParams {
    /// The confirmation question to ask
    pub prompt: String,

    /// Default value if user just presses enter
    #[serde(default)]
    pub default: bool,
}

/// Result from boolean elicitation.
///
/// Returns the user's yes/no confirmation.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ElicitBoolResult {
    /// The boolean value from the user's confirmation
    pub value: bool,
}

impl ElicitBoolResult {
    /// Create a new elicit bool result.
    ///
    /// # Arguments
    ///
    /// * `value` - The boolean confirmation value
    ///
    /// # Returns
    ///
    /// A new `ElicitBoolResult`.
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}
