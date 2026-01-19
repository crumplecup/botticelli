//! Gemini model variants with hierarchical ordering.

use derive_more::Display;
use elicitation::{Prompt, Select};
use strum::EnumIter;
use tracing::instrument;

/// Gemini models ordered from most to least restrictive.
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
pub enum GeminiModel {
    /// Gemini 2.5 Pro - Most restrictive, highest capability
    #[display("gemini-2.5-pro")]
    Gemini25Pro,
    /// Gemini 2.5 Flash - Balanced performance
    #[display("gemini-2.5-flash")]
    Gemini25Flash,
    /// Gemini 2.0 Flash Thinking - Experimental reasoning model
    #[display("gemini-2.0-flash-thinking-exp")]
    Gemini20FlashThinking,
    /// Gemini 2.0 Flash - Previous generation
    #[display("gemini-2.0-flash")]
    Gemini20Flash,
    /// Gemini 2.5 Flash Lite - Least restrictive, fastest
    #[display("gemini-2.5-flash-lite")]
    Gemini25FlashLite,
}

impl GeminiModel {
    /// Get the model string for API calls.
    #[instrument]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gemini25Pro => "gemini-2.5-pro",
            Self::Gemini25Flash => "gemini-2.5-flash",
            Self::Gemini20FlashThinking => "gemini-2.0-flash-thinking-exp",
            Self::Gemini20Flash => "gemini-2.0-flash",
            Self::Gemini25FlashLite => "gemini-2.5-flash-lite",
        }
    }

    /// Get laterally equivalent models in other families.
    ///
    /// Returns tuples of (family, model_string) for models with similar
    /// capability/cost tradeoffs in other providers.
    #[instrument]
    pub fn friends(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Self::Gemini25Pro => vec![
                ("anthropic", "claude-3-5-sonnet-20241022"),
                ("groq", "llama-3.3-70b-versatile"),
            ],
            Self::Gemini25Flash => vec![
                ("groq", "llama-3.1-70b-versatile"),
                ("perplexity", "llama-3.1-sonar-large-128k-online"),
            ],
            Self::Gemini20FlashThinking => vec![
                // Experimental, no direct equivalents yet
            ],
            Self::Gemini20Flash => vec![
                ("groq", "mixtral-8x7b-32768"),
                ("perplexity", "llama-3.1-sonar-small-128k-online"),
            ],
            Self::Gemini25FlashLite => vec![
                ("groq", "llama-3.1-8b-instant"),
                ("perplexity", "llama-3.1-sonar-small-128k-chat"),
            ],
        }
    }

    /// Move up to a more capable/expensive model.
    ///
    /// Returns None if already at the top.
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
    ///
    /// Returns None if already at the bottom.
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
