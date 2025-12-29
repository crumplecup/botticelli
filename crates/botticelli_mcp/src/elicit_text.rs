//! Elicit text tool types for free-form text input.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for eliciting free-form text input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ElicitTextParams {
    /// The prompt to display to the user
    pub prompt: String,
}

/// Result from text elicitation.
///
/// Returns the user's text input directly as a string value.
/// The elicitation crate expects the raw text, not wrapped in an object.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ElicitTextResult {
    /// The text value entered by the user
    pub value: String,
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
    pub fn new(value: String) -> Self {
        Self { value }
    }
}
