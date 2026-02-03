//! Narrative metadata types and TOC structures.

use rmcp::tool;
use elicitation::{Prompt, Select};
use super::TomlAct;
use serde::Deserialize;
use std::collections::HashMap;

/// Root narrative metadata structure (for single narratives).
#[derive(Debug, Clone, Deserialize, derive_getters::Getters, derive_builder::Builder, elicitation::Elicit)]
#[builder(setter(into))]
pub struct TomlNarrative {
    name: String,
    description: String,
    /// Optional template table to use as schema source for content generation
    #[builder(default)]
    template: Option<String>,
    /// Optional target table name for content generation (overrides narrative name)
    #[builder(default)]
    target: Option<String>,
    /// Optional flag to skip content generation (both template and inference modes)
    #[serde(default)]
    #[builder(default)]
    skip_content_generation: bool,
    /// Optional carousel configuration
    #[serde(default)]
    #[builder(default)]
    carousel: Option<crate::CarouselConfig>,
    /// Optional default model for all acts
    #[serde(default)]
    #[builder(default)]
    model: Option<String>,
    /// Optional default temperature for all acts
    #[serde(default)]
    #[builder(default)]
    temperature: Option<f32>,
    /// Optional default max_tokens for all acts
    #[serde(default)]
    #[builder(default)]
    max_tokens: Option<u32>,
    /// Optional budget multipliers
    #[serde(default)]
    #[builder(default)]
    budget: Option<botticelli_core::BudgetConfig>,
}

/// Intermediate structure for deserializing individual [narratives.name] sections.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters, elicitation::Elicit)]
pub struct TomlNarrativeDefinition {
    /// Name is optional here because it comes from the table key [narratives.NAME]
    #[serde(default)]
    name: Option<String>,
    /// Description is now optional
    #[serde(default)]
    description: Option<String>,
    /// Optional template table to use as schema source for content generation
    template: Option<String>,
    /// Optional target table name for content generation (overrides narrative name)
    target: Option<String>,
    /// Optional flag to skip content generation
    #[serde(default)]
    skip_content_generation: bool,
    /// Optional carousel configuration
    #[serde(default)]
    carousel: Option<crate::CarouselConfig>,
    /// Optional default model
    #[serde(default)]
    model: Option<String>,
    /// Optional default temperature
    #[serde(default)]
    temperature: Option<f32>,
    /// Optional default max_tokens
    #[serde(default)]
    max_tokens: Option<u32>,
    /// Optional budget multipliers
    #[serde(default)]
    budget: Option<botticelli_core::BudgetConfig>,
    /// Table of contents for this narrative (just an array of act names)
    toc: Vec<String>,
    /// Optional narrative-specific acts (override shared acts)
    #[serde(default)]
    acts: HashMap<String, TomlAct>,
}

/// Intermediate structure for deserializing the [toc] section (backwards compat).
#[derive(Debug, Clone, Deserialize, elicitation::Elicit)]
#[serde(untagged)]
pub enum TomlToc {
    /// Simple array: toc = ["one", "two"]
    Array(Vec<String>),
    /// Structured: [toc] with order field
    Structured { order: Vec<String> },
}

impl TomlToc {
    /// Get the order vector regardless of variant.
    #[tool]
    pub fn order(&self) -> &[String] {
        match self {
            TomlToc::Array(v) => v,
            TomlToc::Structured { order } => order,
        }
    }
}
