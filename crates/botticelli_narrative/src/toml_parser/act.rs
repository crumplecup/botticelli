//! Act types and conversion logic.

use super::{utils::*, TomlInput, TomlNarrativeFile};
use crate::ActConfig;
use botticelli_core::Input;
use botticelli_error::{NarrativeErrorKind, NarrativeResult};
use serde::Deserialize;
use tracing::{debug, error, instrument};

/// Intermediate structure for deserializing acts.
///
/// Acts can be:
/// - Simple strings: `act_name = "prompt text"`
/// - Resource references: `act_name = "bots.name"` or `act_name = "media.name"`
/// - Arrays: `act_name = ["bots.name", "media.name", "text"]`
/// - Structured tables: `[acts.act_name]` with optional `[[acts.act_name.input]]`
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlAct {
    /// Simple text act or resource reference: `act_name = "prompt"` or `act_name = "bots.name"`
    Simple(String),
    /// Array of references/inputs: `act_name = ["bots.name", "text"]`
    Array(Vec<TomlActInput>),
    /// Structured act with configuration
    Structured(TomlActConfig),
}

/// Input in array syntax - either a reference or inline text.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlActInput {
    /// String reference to resource or plain text
    String(String),
    /// Inline structured input
    Structured(Box<TomlInput>),
}

/// Structured act configuration from TOML.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlActConfig {
    /// Array of inputs via `[[acts.act_name.input]]` syntax
    #[serde(default)]
    pub(super) input: Vec<TomlInput>,

    /// Reference to another narrative to execute as this act
    #[serde(default, alias = "narrative_ref")]
    pub(super) narrative: Option<String>,

    /// Optional model override
    pub(super) model: Option<String>,

    /// Optional temperature override
    pub(super) temperature: Option<f32>,

    /// Optional max_tokens override
    pub(super) max_tokens: Option<u32>,

    /// Optional carousel configuration for this act
    #[serde(default)]
    pub(super) carousel: Option<crate::CarouselConfig>,

    /// Whether to extract and store JSON output (default: only for last act in narrative)
    #[serde(default)]
    pub(super) extract_output: Option<bool>,
}

impl TomlActConfig {
    // Methods moved to TomlAct::to_act_config for multi-narrative file support
}

impl TomlAct {
    /// Convert TOML act to domain ActConfig.
    ///
    /// Requires the parent TomlNarrativeFile for resolving references.
    #[instrument(skip(self, narrative_file))]
    pub fn to_act_config(&self, narrative_file: &TomlNarrativeFile) -> NarrativeResult<ActConfig> {
        debug!("Converting TOML act to domain ActConfig");
        match self {
            TomlAct::Simple(text) => {
                // Check if it's a resource reference
                if is_reference(text) {
                    debug!(reference = %text, "Resolving simple reference");
                    let input = narrative_file.resolve_reference(text)?;
                    Ok(ActConfig::new(vec![input], None, None, None, None, None))
                } else {
                    // Validate that the text is not empty or just whitespace
                    if text.trim().is_empty() {
                        error!("Act prompt cannot be empty or whitespace only");
                        return Err(NarrativeErrorKind::EmptyPrompt("unnamed".to_string()).into());
                    }
                    debug!(text_len = text.len(), "Using simple text act");
                    Ok(ActConfig::from_text(text.clone()))
                }
            }
            TomlAct::Array(items) => {
                debug!(item_count = items.len(), "Processing array act");
                let mut inputs = Vec::new();
                for item in items {
                    match item {
                        TomlActInput::String(s) => {
                            if is_reference(s) {
                                debug!(reference = %s, "Resolving array reference");
                                inputs.push(narrative_file.resolve_reference(s)?);
                            } else {
                                debug!(text_len = s.len(), "Adding array text input");
                                inputs.push(Input::Text(s.clone()));
                            }
                        }
                        TomlActInput::Structured(toml_input) => {
                            // Check if it has a reference field
                            if let Some(ref_str) = &toml_input.reference {
                                debug!(reference = %ref_str, "Resolving structured array reference");
                                inputs.push(narrative_file.resolve_reference(ref_str)?);
                            } else {
                                debug!("Converting structured array input");
                                inputs.push(toml_input.to_input()?);
                            }
                        }
                    }
                }
                debug!(
                    input_count = inputs.len(),
                    "Array act converted successfully"
                );
                Ok(ActConfig::new(inputs, None, None, None, None, None))
            }
            TomlAct::Structured(config) => {
                debug!(
                    input_count = config.input.len(),
                    has_narrative = config.narrative.is_some(),
                    "Processing structured act"
                );

                // Check for narrative reference first (handles mutual exclusivity)
                if let Some(ref narrative_name) = config.narrative {
                    debug!(narrative = %narrative_name, "Creating narrative composition act");
                    return Ok(ActConfig::from_narrative_ref(
                        narrative_name.clone(),
                        config.model.clone(),
                        config.temperature,
                        config.max_tokens,
                    ));
                }

                // Otherwise handle inputs normally
                let mut inputs = Vec::new();
                for toml_input in &config.input {
                    if let Some(ref_str) = &toml_input.reference {
                        debug!(reference = %ref_str, "Resolving structured reference");
                        inputs.push(narrative_file.resolve_reference(ref_str)?);
                    } else {
                        debug!("Converting structured input");
                        inputs.push(toml_input.to_input()?);
                    }
                }
                debug!(
                    input_count = inputs.len(),
                    "Structured act converted successfully"
                );
                Ok(ActConfig::new(
                    inputs,
                    config.model.clone(),
                    config.temperature,
                    config.max_tokens,
                    config.carousel.clone(),
                    config.extract_output,
                ))
            }
        }
    }
}
