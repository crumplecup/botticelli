//! Trait abstraction for narrative configuration providers.
//!
//! This module defines the `NarrativeProvider` trait, which decouples the
//! narrative executor from specific configuration formats (TOML, YAML, JSON, etc.).

use crate::NarrativeMetadata;
use botticelli_core::Input;
use serde::{Deserialize, Serialize};

/// Configuration for a single act in a narrative.
///
/// This structure allows fine-grained control over each act's behavior,
/// including multimodal inputs and per-act model/parameter overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActConfig {
    /// Multimodal inputs for this act.
    ///
    /// Can include text, images, audio, video, documents, or any combination.
    /// Most acts will have a single `Input::Text`, but multimodal acts can
    /// combine multiple input types.
    pub inputs: Vec<Input>,

    /// Optional model override for this specific act.
    ///
    /// If `Some`, this act will use the specified model instead of the
    /// executor's default. Enables per-act model selection.
    ///
    /// Example: `Some("gpt-4".to_string())` or `Some("claude-3-opus-20240229".to_string())`
    pub model: Option<String>,

    /// Optional temperature override for this act.
    ///
    /// Controls randomness/creativity. Typical range: 0.0 (deterministic) to 1.0 (creative).
    pub temperature: Option<f32>,

    /// Optional max_tokens override for this act.
    ///
    /// Limits the length of the generated response.
    pub max_tokens: Option<u32>,
}

impl ActConfig {
    /// Create a simple text-only act configuration.
    ///
    /// Convenience constructor for the common case of a single text prompt
    /// with no model or parameter overrides.
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        Self {
            inputs: vec![Input::Text(text.into())],
            model: None,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Create an act configuration with multimodal inputs.
    pub fn from_inputs(inputs: Vec<Input>) -> Self {
        Self {
            inputs,
            model: None,
            temperature: None,
            max_tokens: None,
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
}
