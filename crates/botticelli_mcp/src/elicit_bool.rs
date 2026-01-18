//! Elicit bool tool types for yes/no confirmation input.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting boolean confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters)]
pub struct ElicitBoolParams {
    /// The confirmation question to ask
    prompt: String,

    /// Default value if user just presses enter
    #[serde(default)]
    default: bool,
}

impl ElicitBoolParams {
    /// Create new elicit bool parameters.
    #[tracing::instrument(skip(prompt), fields(prompt_len = prompt.len()))]
    pub fn new(prompt: String, default: bool) -> Self {
        Self { prompt, default }
    }
}

/// Result from boolean elicitation.
///
/// Returns the user's yes/no confirmation.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters)]
pub struct ElicitBoolResult {
    /// The boolean value from the user's confirmation
    value: bool,
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
    #[tracing::instrument]
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}
