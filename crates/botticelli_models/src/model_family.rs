//! Model family enums and fallback ordering.
//!
//! Defines model families (Gemini, Groq, etc.) with fallback ordering for
//! "friendly" movement when switching between providers.

use derive_more::Display;
use strum::EnumIter;

/// Model families supported by Botticelli.
///
/// Variants are ordered by fallback preference. When a model is unavailable,
/// the system will try equivalent models in other families following this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter)]
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

#[cfg(test)]
mod model_family_test {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_fallback_order_excludes_self() {
        for family in ModelFamily::iter() {
            let fallbacks = family.fallback_order();
            assert!(!fallbacks.contains(&family), "Fallback should not include self");
        }
    }

    #[test]
    fn test_fallback_order_starts_after_current() {
        let order = ModelFamily::Gemini.fallback_order();
        assert_eq!(order[0], ModelFamily::Groq);
        
        let order = ModelFamily::Groq.fallback_order();
        assert_eq!(order[0], ModelFamily::Perplexity);
    }

    #[test]
    fn test_fallback_order_wraps() {
        let order = ModelFamily::Ollama.fallback_order();
        assert_eq!(order[0], ModelFamily::Gemini);
    }
}
