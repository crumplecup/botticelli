//! Elicit text tool types for free-form text input.

use derive_getters::Getters;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting free-form text input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, derive_new::new)]
pub struct ElicitTextParams {
    /// The prompt to display to the user
    prompt: String,
}

/// Result from text elicitation.
///
/// Returns the user's text input directly as a string value.
/// The elicitation crate expects the raw text, not wrapped in an object.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters, derive_new::new)]
pub struct ElicitTextResult {
    /// The text value entered by the user
    value: String,
}
