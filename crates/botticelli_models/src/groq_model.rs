//! Groq model variants with hierarchical ordering.

use derive_more::Display;
use strum::EnumIter;

/// Groq models ordered from most to least restrictive.
///
/// Variants are ordered by rate limits and cost, allowing "loyal" movement
/// up (more capable/expensive) or down (faster/cheaper) within the family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter)]
pub enum GroqModel {
    /// Llama 3.3 70B Versatile - Most capable
    #[display("llama-3.3-70b-versatile")]
    Llama33_70BVersatile,
    /// Llama 3.1 70B Versatile - Previous generation large
    #[display("llama-3.1-70b-versatile")]
    Llama31_70BVersatile,
    /// Mixtral 8x7B - Efficient mixture-of-experts
    #[display("mixtral-8x7b-32768")]
    Mixtral8x7B,
    /// Llama 3.1 8B Instant - Fastest, least restrictive
    #[display("llama-3.1-8b-instant")]
    Llama31_8BInstant,
}

impl GroqModel {
    /// Get the model string for API calls.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Llama33_70BVersatile => "llama-3.3-70b-versatile",
            Self::Llama31_70BVersatile => "llama-3.1-70b-versatile",
            Self::Mixtral8x7B => "mixtral-8x7b-32768",
            Self::Llama31_8BInstant => "llama-3.1-8b-instant",
        }
    }

    /// Get laterally equivalent models in other families.
    pub fn friends(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Self::Llama33_70BVersatile => vec![
                ("gemini", "gemini-2.5-pro"),
                ("anthropic", "claude-3-5-sonnet-20241022"),
            ],
            Self::Llama31_70BVersatile => vec![
                ("gemini", "gemini-2.5-flash"),
                ("perplexity", "llama-3.1-sonar-large-128k-online"),
            ],
            Self::Mixtral8x7B => vec![
                ("gemini", "gemini-2.0-flash"),
                ("perplexity", "llama-3.1-sonar-small-128k-online"),
            ],
            Self::Llama31_8BInstant => vec![
                ("gemini", "gemini-2.5-flash-lite"),
                ("perplexity", "llama-3.1-sonar-small-128k-chat"),
            ],
        }
    }

    /// Move up to a more capable/expensive model.
    pub fn move_up(&self) -> Option<Self> {
        use strum::IntoEnumIterator;
        let all: Vec<_> = Self::iter().collect();
        let current_idx = all.iter().position(|m| m == self)?;
        
        if current_idx == 0 {
            None
        } else {
            Some(all[current_idx - 1])
        }
    }

    /// Move down to a faster/cheaper model.
    pub fn move_down(&self) -> Option<Self> {
        use strum::IntoEnumIterator;
        let all: Vec<_> = Self::iter().collect();
        let current_idx = all.iter().position(|m| m == self)?;
        
        if current_idx == all.len() - 1 {
            None
        } else {
            Some(all[current_idx + 1])
        }
    }
}

#[cfg(test)]
mod groq_model_test {
    use super::*;

    #[test]
    fn test_move_up_from_middle() {
        assert_eq!(
            GroqModel::Mixtral8x7B.move_up(),
            Some(GroqModel::Llama31_70BVersatile)
        );
    }

    #[test]
    fn test_move_up_from_top() {
        assert_eq!(GroqModel::Llama33_70BVersatile.move_up(), None);
    }

    #[test]
    fn test_move_down_from_middle() {
        assert_eq!(
            GroqModel::Mixtral8x7B.move_down(),
            Some(GroqModel::Llama31_8BInstant)
        );
    }

    #[test]
    fn test_move_down_from_bottom() {
        assert_eq!(GroqModel::Llama31_8BInstant.move_down(), None);
    }

    #[test]
    fn test_as_str_matches_display() {
        use strum::IntoEnumIterator;
        for model in GroqModel::iter() {
            assert_eq!(model.as_str(), model.to_string());
        }
    }
}
