//! Trait abstraction for narrative configuration providers.
//!
//! This module defines the `NarrativeProvider` trait, which decouples the
//! narrative executor from specific configuration formats (TOML, YAML, JSON, etc.).

/// Trait for providing narrative configuration and metadata.
///
/// Implementations can load narratives from any source (files, databases, APIs).
/// The trait uses associated types to remain independent of concrete implementations.
pub trait NarrativeProvider: Send + Sync {
    /// Type representing narrative metadata (name, description, template).
    type Metadata;

    /// Type representing act configuration (inputs, model settings, etc.).
    type ActConfig;

    /// Type representing carousel configuration for repeated execution.
    type CarouselConfig;

    /// Name of the narrative for tracking and identification.
    fn name(&self) -> &str;

    /// Narrative metadata including name, description, and template.
    fn metadata(&self) -> &Self::Metadata;

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
    fn get_act_config(&self, act_name: &str) -> Option<Self::ActConfig>;

    /// Resolve a referenced narrative by name for narrative composition.
    ///
    /// For multi-narrative files, this returns the referenced narrative.
    /// For single-narrative files, this returns `None`.
    ///
    /// # Arguments
    ///
    /// * `narrative_name` - Name of the narrative to resolve
    ///
    /// # Returns
    ///
    /// Returns the referenced narrative if it exists, `None` otherwise.
    fn resolve_narrative(&self, _narrative_name: &str) -> Option<&dyn NarrativeProvider<
        Metadata = Self::Metadata,
        ActConfig = Self::ActConfig,
        CarouselConfig = Self::CarouselConfig,
    >> {
        None // Default implementation for single narratives
    }

    /// Get the carousel configuration if present.
    ///
    /// Returns `None` if this narrative doesn't have carousel configuration.
    fn carousel_config(&self) -> Option<&Self::CarouselConfig> {
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
