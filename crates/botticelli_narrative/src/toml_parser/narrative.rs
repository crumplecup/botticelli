//! Narrative metadata types and TOC structures.

use super::TomlAct;
use serde::Deserialize;
use std::collections::HashMap;

/// Root narrative metadata structure (for single narratives).
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlNarrative {
    pub(super) name: String,
    pub(super) description: String,
    /// Optional template table to use as schema source for content generation
    pub(super) template: Option<String>,
    /// Optional target table name for content generation (overrides narrative name)
    pub(super) target: Option<String>,
    /// Optional flag to skip content generation (both template and inference modes)
    #[serde(default)]
    pub(super) skip_content_generation: bool,
    /// Optional carousel configuration
    #[serde(default)]
    pub(super) carousel: Option<crate::CarouselConfig>,
    /// Optional default model for all acts
    #[serde(default)]
    pub(super) model: Option<String>,
    /// Optional default temperature for all acts
    #[serde(default)]
    pub(super) temperature: Option<f32>,
    /// Optional default max_tokens for all acts
    #[serde(default)]
    pub(super) max_tokens: Option<u32>,
    /// Optional budget multipliers
    #[serde(default)]
    pub(super) budget: Option<botticelli_core::BudgetConfig>,
}

/// Intermediate structure for deserializing individual [narratives.name] sections.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlNarrativeDefinition {
    /// Name is optional here because it comes from the table key [narratives.NAME]
    #[serde(default)]
    pub(super) name: Option<String>,
    /// Description is now optional
    #[serde(default)]
    pub(super) description: Option<String>,
    /// Optional template table to use as schema source for content generation
    pub(super) template: Option<String>,
    /// Optional target table name for content generation (overrides narrative name)
    pub(super) target: Option<String>,
    /// Optional flag to skip content generation
    #[serde(default)]
    pub(super) skip_content_generation: bool,
    /// Optional carousel configuration
    #[serde(default)]
    pub(super) carousel: Option<crate::CarouselConfig>,
    /// Optional default model
    #[serde(default)]
    pub(super) model: Option<String>,
    /// Optional default temperature
    #[serde(default)]
    pub(super) temperature: Option<f32>,
    /// Optional default max_tokens
    #[serde(default)]
    pub(super) max_tokens: Option<u32>,
    /// Optional budget multipliers
    #[serde(default)]
    pub(super) budget: Option<botticelli_core::BudgetConfig>,
    /// Table of contents for this narrative (just an array of act names)
    pub(super) toc: Vec<String>,
    /// Optional narrative-specific acts (override shared acts)
    #[serde(default)]
    pub(super) acts: HashMap<String, TomlAct>,
}

/// Intermediate structure for deserializing the [toc] section (backwards compat).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlToc {
    /// Simple array: toc = ["one", "two"]
    Array(Vec<String>),
    /// Structured: [toc] with order field
    Structured { order: Vec<String> },
}

impl TomlToc {
    /// Get the order vector regardless of variant.
    pub fn order(&self) -> &[String] {
        match self {
            TomlToc::Array(v) => v,
            TomlToc::Structured { order } => order,
        }
    }
}
