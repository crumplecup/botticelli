//! Configuration types for narrative acts.
//!
//! This module defines `ActConfig` which configures individual acts in a narrative.
//! The `NarrativeProvider` trait is defined in `botticelli_interface` and should be
//! imported from there.

use crate::CarouselConfig;
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

    /// Whether to extract and store JSON output from this act's response.
    ///
    /// If `None`, defaults to true only for the last act in the narrative.
    /// Set to `true` to force extraction for intermediate acts.
    /// Set to `false` to skip extraction even for the last act.
    #[serde(default)]
    extract_output: Option<bool>,
}

impl ActConfig {
    /// Create a new act configuration with all fields.
    #[tracing::instrument(skip(inputs), fields(
        input_count = inputs.len(),
        has_model = model.is_some(),
        has_temperature = temperature.is_some(),
        has_max_tokens = max_tokens.is_some(),
        has_carousel = carousel.is_some(),
        has_extract_output = extract_output.is_some()
    ))]
    pub fn new(
        inputs: Vec<Input>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        carousel: Option<CarouselConfig>,
        extract_output: Option<bool>,
    ) -> Self {
        tracing::debug!("Creating ActConfig");
        Self {
            inputs,
            narrative_ref: None,
            model,
            temperature,
            max_tokens,
            carousel,
            extract_output,
        }
    }

    /// Create an act that references another narrative.
    #[tracing::instrument(fields(
        narrative_name,
        has_model = model.is_some(),
        has_temperature = temperature.is_some(),
        has_max_tokens = max_tokens.is_some()
    ))]
    pub fn from_narrative_ref<S: Into<String>>(
        narrative_name: S,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Self {
        let name = narrative_name.into();
        tracing::debug!(narrative_name = %name, "Creating narrative reference ActConfig");
        Self {
            inputs: Vec::new(),
            narrative_ref: Some(name),
            model,
            temperature,
            max_tokens,
            carousel: None,
            extract_output: None,
        }
    }

    /// Create a simple text-only act configuration.
    ///
    /// Convenience constructor for the common case of a single text prompt
    /// with no model or parameter overrides.
    #[tracing::instrument(skip(text), fields(text_len))]
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        let text_string = text.into();
        tracing::Span::current().record("text_len", text_string.len());
        tracing::debug!("Creating text-only ActConfig");
        Self {
            inputs: vec![Input::Text(text_string)],
            narrative_ref: None,
            model: None,
            temperature: None,
            max_tokens: None,
            carousel: None,
            extract_output: None,
        }
    }

    /// Check if this act is a narrative reference.
    pub fn is_narrative_ref(&self) -> bool {
        self.narrative_ref.is_some()
    }

    /// Create an act configuration with multimodal inputs.
    #[tracing::instrument(skip(inputs), fields(input_count = inputs.len()))]
    pub fn from_inputs(inputs: Vec<Input>) -> Self {
        tracing::debug!("Creating multimodal ActConfig");
        Self {
            inputs,
            narrative_ref: None,
            model: None,
            temperature: None,
            max_tokens: None,
            carousel: None,
            extract_output: None,
        }
    }

    /// Builder method to set the model override.
    #[tracing::instrument(skip(self, model), fields(model_name))]
    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        let model_string = model.into();
        tracing::Span::current().record("model_name", &model_string);
        tracing::debug!("Setting model override");
        self.model = Some(model_string);
        self
    }

    /// Builder method to set the temperature override.
    #[tracing::instrument(skip(self), fields(temperature))]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        tracing::debug!("Setting temperature override");
        self.temperature = Some(temperature);
        self
    }

    /// Builder method to set the max_tokens override.
    #[tracing::instrument(skip(self), fields(max_tokens))]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        tracing::debug!("Setting max_tokens override");
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Builder method to set the carousel configuration.
    #[tracing::instrument(skip(self, carousel), fields(has_carousel = true))]
    pub fn with_carousel(mut self, carousel: CarouselConfig) -> Self {
        tracing::debug!("Setting carousel configuration");
        self.carousel = Some(carousel);
        self
    }

    /// Builder method to set the inputs.
    #[tracing::instrument(skip(self, inputs), fields(input_count = inputs.len()))]
    pub fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
        tracing::debug!("Setting inputs");
        self.inputs = inputs;
        self
    }

    /// Set inputs in-place (mutable setter).
    ///
    /// This is used for runtime modifications to act configurations,
    /// such as injecting user prompts.
    #[tracing::instrument(skip(self, inputs), fields(input_count = inputs.len()))]
    pub fn set_inputs(&mut self, inputs: Vec<Input>) {
        tracing::debug!("Mutating inputs");
        self.inputs = inputs;
    }

    /// Set max_tokens in-place (mutable setter).
    ///
    /// This is used for runtime overrides to act configurations.
    #[tracing::instrument(skip(self))]
    pub fn set_max_tokens(&mut self, max_tokens: Option<u32>) {
        tracing::debug!(max_tokens = ?max_tokens, "Mutating max_tokens");
        self.max_tokens = max_tokens;
    }
}
