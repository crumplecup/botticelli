//! Elicit bool tool types for yes/no confirmation input.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting boolean confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, derive_new::new)]
pub struct ElicitBoolParams {
    /// The confirmation question to ask
    prompt: String,

    /// Default value if user just presses enter
    #[serde(default)]
    default: bool,
}

/// Result from boolean elicitation.
///
/// Returns the user's yes/no confirmation.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters, derive_new::new)]
pub struct ElicitBoolResult {
    /// The boolean value from the user's confirmation
    value: bool,
}
