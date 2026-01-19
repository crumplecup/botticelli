//! Model family enums and fallback ordering.
//!
//! Defines model families (Gemini, Groq, etc.) with fallback ordering for
//! "friendly" movement when switching between providers.

use derive_more::Display;
use elicitation::{Prompt, Select};
use strum::EnumIter;
use tracing::instrument;

/// Model families supported by Botticelli.
///
/// Variants are ordered by fallback preference. When a model is unavailable,
/// the system will try equivalent models in other families following this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, elicitation::Elicit)]
pub enum ModelFamily {
    /// Google Gemini models (default/preferred)
    #[display("gemini")]
    Gemini,
    /// Groq LPU inference models
    #[display("groq")]
    Groq,
    /// Perplexity models
    #[display("perplexity")]
    Perplexity,
    /// Anthropic Claude models
    #[display("anthropic")]
    Anthropic,
    /// Local Ollama models (last resort)
    #[display("ollama")]
    Ollama,
}

impl ModelFamily {
    /// Get the default fallback order starting after this family.
    ///
    /// Returns families in enum order, wrapping around if needed.
    #[instrument]
    pub fn fallback_order(&self) -> Vec<ModelFamily> {
        use strum::IntoEnumIterator;
        let all: Vec<_> = ModelFamily::iter().collect();
        let current_idx = all.iter().position(|f| f == self).unwrap_or(0);

        // Start after current, wrap around, exclude current
        let mut result = Vec::new();
        for i in 1..all.len() {
            let idx = (current_idx + i) % all.len();
            result.push(all[idx]);
        }
        result
    }
}
