//! Model selection with boundary constraints and fallback logic.

use derive_more::Display;
use derive_new::new;
use tracing::instrument;

use crate::{GeminiModel, GroqModel, ModelFamily, RateLimitDetector};

/// Boundary constraints for model selection.
///
/// Defines upper and lower bounds to prevent using models that are
/// too expensive/slow (upper bound) or too cheap/fast (lower bound).
#[derive(Debug, Clone, Copy, PartialEq, Eq, new, serde::Serialize, serde::Deserialize)]
pub struct ModelBounds {
    /// Minimum acceptable model (None = no lower bound)
    lower: Option<ModelId>,
    /// Maximum acceptable model (None = no upper bound)
    upper: Option<ModelId>,
}

impl ModelBounds {
    /// No boundaries - accept any model.
    pub fn none() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    /// Set lower bound only.
    pub fn lower_bound(model: ModelId) -> Self {
        Self {
            lower: Some(model),
            upper: None,
        }
    }

    /// Set upper bound only.
    pub fn upper_bound(model: ModelId) -> Self {
        Self {
            lower: None,
            upper: Some(model),
        }
    }

    /// Set both bounds.
    pub fn both(lower: ModelId, upper: ModelId) -> Self {
        Self {
            lower: Some(lower),
            upper: Some(upper),
        }
    }

    /// Check if a model is within bounds.
    #[instrument]
    pub fn allows(&self, model: ModelId) -> bool {
        if let Some(lower) = self.lower
            && !model.is_at_least(lower)
        {
            return false;
        }
        if let Some(upper) = self.upper
            && !model.is_at_most(upper)
        {
            return false;
        }
        true
    }

    /// Get the lower bound.
    pub fn lower(&self) -> Option<ModelId> {
        self.lower
    }

    /// Get the upper bound.
    pub fn upper(&self) -> Option<ModelId> {
        self.upper
    }
}

/// Unified model identifier across all families.
///
/// Allows comparing and ordering models regardless of provider.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Display, serde::Serialize, serde::Deserialize,
)]
pub enum ModelId {
    /// Gemini model variant
    #[display("gemini:{}", _0)]
    Gemini(GeminiModel),
    /// Groq model variant
    #[display("groq:{}", _0)]
    Groq(GroqModel),
}

impl std::str::FromStr for ModelId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (family, model) = s
            .split_once(':')
            .ok_or_else(|| format!("Invalid model format: '{}' (expected 'family:model')", s))?;

        match family {
            "gemini" => GeminiModel::from_str(model)
                .map(Self::Gemini)
                .ok_or_else(|| format!("Unknown Gemini model: '{}'", model)),
            "groq" => GroqModel::from_str(model)
                .map(Self::Groq)
                .ok_or_else(|| format!("Unknown Groq model: '{}'", model)),
            _ => Err(format!("Unknown model family: '{}'", family)),
        }
    }
}

impl ModelId {
    /// Get the family this model belongs to.
    #[instrument]
    pub fn family(&self) -> ModelFamily {
        match self {
            Self::Gemini(_) => ModelFamily::Gemini,
            Self::Groq(_) => ModelFamily::Groq,
        }
    }

    /// Get the model string for API calls.
    #[instrument]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gemini(m) => m.as_str(),
            Self::Groq(m) => m.as_str(),
        }
    }

    /// Get "friends" - laterally equivalent models in other families.
    #[instrument]
    pub fn friends(&self) -> Vec<Self> {
        match self {
            Self::Gemini(m) => m
                .friends()
                .into_iter()
                .filter_map(|(family, model_str)| match family {
                    "groq" => GroqModel::from_str(model_str).map(Self::Groq),
                    _ => None,
                })
                .collect(),
            Self::Groq(m) => m
                .friends()
                .into_iter()
                .filter_map(|(family, model_str)| match family {
                    "gemini" => GeminiModel::from_str(model_str).map(Self::Gemini),
                    _ => None,
                })
                .collect(),
        }
    }

    /// Move up to more capable/expensive model within family.
    #[instrument]
    pub fn move_up(&self) -> Option<Self> {
        match self {
            Self::Gemini(m) => m.move_up().map(Self::Gemini),
            Self::Groq(m) => m.move_up().map(Self::Groq),
        }
    }

    /// Move down to faster/cheaper model within family.
    #[instrument]
    pub fn move_down(&self) -> Option<Self> {
        match self {
            Self::Gemini(m) => m.move_down().map(Self::Gemini),
            Self::Groq(m) => m.move_down().map(Self::Groq),
        }
    }

    /// Check if this model is at least as capable as another.
    ///
    /// For same-family comparisons, compares tier positions.
    /// Cross-family comparisons use the friends() equivalence mapping.
    #[instrument]
    pub fn is_at_least(&self, other: Self) -> bool {
        if self.family() == other.family() {
            // Same family: compare positions directly
            self.family_position() <= other.family_position()
        } else {
            // Cross-family: use friends mapping
            self.friends().contains(&other) || self.is_higher_tier_than(other)
        }
    }

    /// Check if this model is at most as capable as another.
    #[instrument]
    pub fn is_at_most(&self, other: Self) -> bool {
        if self.family() == other.family() {
            self.family_position() >= other.family_position()
        } else {
            self.friends().contains(&other) || !self.is_higher_tier_than(other)
        }
    }

    /// Get position within family (0 = most capable).
    fn family_position(&self) -> usize {
        match self {
            Self::Gemini(GeminiModel::Gemini25Pro) => 0,
            Self::Gemini(GeminiModel::Gemini25Flash) => 1,
            Self::Gemini(GeminiModel::Gemini20FlashThinking) => 2,
            Self::Gemini(GeminiModel::Gemini20Flash) => 3,
            Self::Gemini(GeminiModel::Gemini25FlashLite) => 4,
            Self::Groq(GroqModel::Llama33_70BVersatile) => 0,
            Self::Groq(GroqModel::Llama31_70BVersatile) => 1,
            Self::Groq(GroqModel::Mixtral8x7B) => 2,
            Self::Groq(GroqModel::Llama31_8BInstant) => 3,
        }
    }

    /// Rough cross-family tier comparison.
    fn is_higher_tier_than(&self, other: Self) -> bool {
        // Simplified: compare family positions as proxy
        self.family_position() < other.family_position()
    }
}

// String parsing helpers for ModelId construction
impl GeminiModel {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "gemini-2.5-pro" => Some(Self::Gemini25Pro),
            "gemini-2.5-flash" => Some(Self::Gemini25Flash),
            "gemini-2.0-flash-thinking-exp" => Some(Self::Gemini20FlashThinking),
            "gemini-2.0-flash" => Some(Self::Gemini20Flash),
            "gemini-2.5-flash-lite" => Some(Self::Gemini25FlashLite),
            _ => None,
        }
    }
}

impl GroqModel {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "llama-3.3-70b-versatile" => Some(Self::Llama33_70BVersatile),
            "llama-3.1-70b-versatile" => Some(Self::Llama31_70BVersatile),
            "mixtral-8x7b-32768" => Some(Self::Mixtral8x7B),
            "llama-3.1-8b-instant" => Some(Self::Llama31_8BInstant),
            _ => None,
        }
    }
}

/// Selection strategy for fallback behavior.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
pub enum SelectionStrategy {
    /// Try loyal movement first (within family), then friendly (cross-family)
    LoyalFirst,
    /// Try friendly movement first (cross-family), then loyal (within family)
    #[serde(rename = "friendly_first")]
    #[default]
    FriendlyFirst,
}

/// Orchestrates model selection with fallback logic.
#[derive(Debug, Clone, new)]
pub struct ModelSelector {
    bounds: ModelBounds,
    strategy: SelectionStrategy,
    detector: RateLimitDetector,
}

impl ModelSelector {
    /// Select next model after rate limit error.
    ///
    /// Returns None if error is not rate-related or no valid fallback exists.
    #[instrument(skip(self))]
    pub fn select_next(&mut self, current: ModelId, error: &str) -> Option<ModelId> {
        if !self.detector.is_rate_limit_message(error) {
            return None;
        }

        self.detector.record(current.family(), current);

        match self.strategy {
            SelectionStrategy::LoyalFirst => self
                .try_loyal_movement(current)
                .or_else(|| self.try_friendly_movement(current)),
            SelectionStrategy::FriendlyFirst => self
                .try_friendly_movement(current)
                .or_else(|| self.try_loyal_movement(current)),
        }
    }

    /// Get current rate limit status for a family.
    #[instrument(skip(self))]
    pub fn get_status(&self, family: ModelFamily) -> Option<&crate::RateLimitStatus> {
        self.detector.get_status(family)
    }

    fn try_loyal_movement(&self, current: ModelId) -> Option<ModelId> {
        current.move_down().filter(|&next| self.bounds.allows(next))
    }

    fn try_friendly_movement(&self, current: ModelId) -> Option<ModelId> {
        current.friends().into_iter().find(|&friend| {
            self.bounds.allows(friend) && !self.detector.is_rate_limited(friend.family())
        })
    }
}

