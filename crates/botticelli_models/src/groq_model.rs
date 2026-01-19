//! Groq model variants with hierarchical ordering.

use derive_more::Display;
use elicitation::{Prompt, Select};
use strum::EnumIter;
use tracing::instrument;

/// Groq models ordered from most to least restrictive.
///
/// Variants are ordered by rate limits and cost, allowing "loyal" movement
/// up (more capable/expensive) or down (faster/cheaper) within the family.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumIter,
    serde::Serialize,
    serde::Deserialize,
    elicitation::Elicit,
)]
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
    #[instrument]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Llama33_70BVersatile => "llama-3.3-70b-versatile",
            Self::Llama31_70BVersatile => "llama-3.1-70b-versatile",
            Self::Mixtral8x7B => "mixtral-8x7b-32768",
            Self::Llama31_8BInstant => "llama-3.1-8b-instant",
        }
    }

    /// Get laterally equivalent models in other families.
    #[instrument]
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
    #[instrument]
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
    #[instrument]
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
