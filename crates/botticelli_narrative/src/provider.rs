//! Trait abstraction for narrative configuration providers.
//!
//! This module defines the `NarrativeProvider` trait, which decouples the
//! narrative executor from specific configuration formats (TOML, YAML, JSON, etc.).

use crate::{CarouselConfig, NarrativeMetadata};
use botticelli_core::Input;
use serde::{Deserialize, Serialize};

/// Configuration for a single act in a narrative.
///
/// This structure allows fine-grained control over each act's behavior,
/// including multimodal inputs and per-act model/parameter overrides.
///
/// Acts can either:
/// - Have direct inputs (traditional act execution)
/// - Reference another narrative (narrative composition)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, derive_getters::Getters)]
pub struct ActConfig {
    /// Multimodal inputs for this act.
    ///
    /// Can include text, images, audio, video, documents, or any combination.
    /// Most acts will have a single `Input::Text`, but multimodal acts can
    /// combine multiple input types.
    ///
    /// Mutually exclusive with `narrative_ref`.
    #[serde(default)]
    inputs: Vec<Input>,

    /// Reference to another narrative to execute as this act.
    ///
    /// When set, this act will execute the referenced narrative and use its
    /// output as the act's result. Enables narrative composition.
    ///
    /// Mutually exclusive with `inputs`.
    #[serde(default)]
    narrative_ref: Option<String>,

    /// Optional model override for this specific act.
    ///
    /// If `Some`, this act will use the specified model instead of the
    /// executor's default. Enables per-act model selection.
    ///
    /// Example: `Some("gpt-4".to_string())` or `Some("claude-3-opus-20240229".to_string())`
    model: Option<String>,

    /// Optional temperature override for this act.
    ///
    /// Controls randomness/creativity. Typical range: 0.0 (deterministic) to 1.0 (creative).
    temperature: Option<f32>,

    /// Optional max_tokens override for this act.
    ///
    /// Limits the length of the generated response.
    max_tokens: Option<u32>,

    /// Optional carousel configuration for repeated execution.
    ///
    /// If `Some`, this act will be executed multiple times according to the
    /// carousel configuration, with rate limit budgeting applied.
    carousel: Option<CarouselConfig>,
}

impl ActConfig {
    /// Create a new act configuration with all fields.
    pub fn new(
        inputs: Vec<Input>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        carousel: Option<CarouselConfig>,
    ) -> Self {
        Self {
            inputs,
            narrative_ref: None,
            model,
            temperature,
            max_tokens,
            carousel,
        }
    }
    
    /// Create an act that references another narrative.
    pub fn from_narrative_ref<S: Into<String>>(
        narrative_name: S,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Self {
        Self {
            inputs: Vec::new(),
            narrative_ref: Some(narrative_name.into()),
            model,
            temperature,
            max_tokens,
            carousel: None,
        }
    }

    /// Create a simple text-only act configuration.
    ///
    /// Convenience constructor for the common case of a single text prompt
    /// with no model or parameter overrides.
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        Self {
            inputs: vec![Input::Text(text.into())],
            narrative_ref: None,
            model: None,
            temperature: None,
            max_tokens: None,
            carousel: None,
        }
    }

    /// Check if this act is a narrative reference.
    pub fn is_narrative_ref(&self) -> bool {
        self.narrative_ref.is_some()
    }
    
    /// Create an act configuration with multimodal inputs.
    pub fn from_inputs(inputs: Vec<Input>) -> Self {
        Self {
            inputs,
            narrative_ref: None,
            model: None,
            temperature: None,
            max_tokens: None,
            carousel: None,
        }
    }

    /// Builder method to set the model override.
    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Builder method to set the temperature override.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Builder method to set the max_tokens override.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Builder method to set the carousel configuration.
    pub fn with_carousel(mut self, carousel: CarouselConfig) -> Self {
        self.carousel = Some(carousel);
        self
    }

    /// Builder method to set the inputs.
    pub fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
        self.inputs = inputs;
        self
    }
}

/// Provides access to narrative configuration data.
///
/// This trait abstracts over different configuration sources (TOML files,
/// YAML, JSON, databases, etc.), allowing the executor to work with any
/// implementation.
///
/// By programming to this interface rather than concrete types, we achieve:
/// - Format flexibility (easy to add new config formats)
/// - Better testability (simple mock implementations)
/// - Reduced coupling (config changes don't ripple through executor)
/// - Multimodal support (acts can use text, images, audio, video, documents)
/// - Per-act model selection (different acts can use different LLMs)
pub trait NarrativeProvider {
    /// Name of the narrative for tracking and identification.
    fn name(&self) -> &str;

    /// Narrative metadata including name, description, and template.
    fn metadata(&self) -> &NarrativeMetadata;

    /// Ordered list of act names to execute in sequence.
    ///
    /// The executor will process acts in this exact order.
    fn act_names(&self) -> &[String];

    /// Get the configuration for a specific act.
    ///
    /// Returns `None` if the act doesn't exist.
    ///
    /// The configuration includes:
    /// - Multimodal inputs (text, images, audio, etc.)
    /// - Optional model override
    /// - Optional temperature/max_tokens overrides
    fn get_act_config(&self, act_name: &str) -> Option<ActConfig>;

    /// Get the carousel configuration if present.
    ///
    /// Returns `None` if this narrative doesn't have carousel configuration.
    fn carousel_config(&self) -> Option<&crate::CarouselConfig> {
        None
    }

    /// Get the source file path for this narrative.
    ///
    /// Used to resolve relative paths in nested narratives.
    /// Returns `None` if the narrative wasn't loaded from a file.
    fn source_path(&self) -> Option<&std::path::Path> {
        None
    }
}
