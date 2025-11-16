//! Core data structures for narratives.

use crate::narrative::{ActConfig, NarrativeError, NarrativeErrorKind, NarrativeProvider};
use crate::narrative::toml as toml_parsing;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

/// Narrative metadata from the `[narration]` section.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct NarrativeMetadata {
    /// Unique identifier for this narrative
    pub name: String,
    /// Human-readable description of what this narrative does
    pub description: String,
    /// Optional template table to use as schema source for content generation
    pub template: Option<String>,
}

/// Table of contents from the `[toc]` section.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct NarrativeToc {
    /// Ordered list of act names to execute
    pub order: Vec<String>,
}

/// Complete narrative structure parsed from TOML.
///
/// # Example TOML Structure
///
/// Simple text acts:
/// ```toml
/// [narration]
/// name = "example"
/// description = "An example narrative"
///
/// [toc]
/// order = ["act1", "act2"]
///
/// [acts]
/// act1 = "First prompt"
/// act2 = "Second prompt"
/// ```
///
/// Structured multimodal acts:
/// ```toml
/// [acts.analyze]
/// model = "gemini-pro-vision"
/// temperature = 0.3
///
/// [[acts.analyze.input]]
/// type = "text"
/// content = "Analyze this image"
///
/// [[acts.analyze.input]]
/// type = "image"
/// mime = "image/png"
/// url = "https://example.com/image.png"
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Narrative {
    /// Narrative metadata
    pub metadata: NarrativeMetadata,

    /// Table of contents defining execution order
    pub toc: NarrativeToc,

    /// Map of act names to their configurations
    #[serde(skip)]
    pub acts: HashMap<String, ActConfig>,
}

impl Narrative {
    /// Loads a narrative from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML is invalid
    /// - Validation fails (missing acts, empty order, etc.)
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, NarrativeError> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            NarrativeError::new(NarrativeErrorKind::FileRead(e.to_string()), line!(), file!())
        })?;

        content.parse()
    }

    /// Validates the narrative structure.
    ///
    /// Ensures:
    /// - Table of contents is not empty
    /// - All acts referenced in toc exist in the acts map
    /// - All acts have at least one input
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn validate(&self) -> Result<(), NarrativeError> {
        // Check that toc.order is not empty
        if self.toc.order.is_empty() {
            return Err(NarrativeError::new(
                NarrativeErrorKind::EmptyToc,
                line!(),
                file!(),
            ));
        }

        // Check that all acts in toc.order exist in acts map
        for act_name in &self.toc.order {
            if !self.acts.contains_key(act_name) {
                return Err(NarrativeError::new(
                    NarrativeErrorKind::MissingAct(act_name.clone()),
                    line!(),
                    file!(),
                ));
            }
        }

        // Check that all acts have at least one input
        for (act_name, config) in &self.acts {
            if config.inputs.is_empty() {
                return Err(NarrativeError::new(
                    NarrativeErrorKind::EmptyPrompt(act_name.clone()),
                    line!(),
                    file!(),
                ));
            }
        }

        Ok(())
    }

    /// Returns the acts in the order specified by the table of contents.
    ///
    /// Each tuple contains the act name and its configuration.
    pub fn ordered_acts(&self) -> Vec<(&str, &ActConfig)> {
        self.toc
            .order
            .iter()
            .filter_map(|name| {
                self.acts
                    .get(name)
                    .map(|config| (name.as_str(), config))
            })
            .collect()
    }
}

impl FromStr for Narrative {
    type Err = NarrativeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse TOML into intermediate structure
        let toml_narrative: toml_parsing::TomlNarrative = toml::from_str(s).map_err(|e| {
            NarrativeError::new(NarrativeErrorKind::TomlParse(e.to_string()), line!(), file!())
        })?;

        // Convert to domain types
        let metadata = NarrativeMetadata {
            name: toml_narrative.narration.name,
            description: toml_narrative.narration.description,
            template: toml_narrative.narration.template,
        };

        let toc = NarrativeToc {
            order: toml_narrative.toc.order,
        };

        let mut acts = HashMap::new();
        for (act_name, toml_act) in toml_narrative.acts {
            let act_config = toml_act.to_act_config().map_err(|e| {
                // Check if this is an empty prompt error
                if e.contains("empty") || e.contains("whitespace") {
                    NarrativeError::new(
                        NarrativeErrorKind::EmptyPrompt(act_name.clone()),
                        line!(),
                        file!(),
                    )
                } else {
                    NarrativeError::new(
                        NarrativeErrorKind::TomlParse(format!("Act '{}': {}", act_name, e)),
                        line!(),
                        file!(),
                    )
                }
            })?;
            acts.insert(act_name, act_config);
        }

        let narrative = Narrative { metadata, toc, acts };
        narrative.validate()?;
        Ok(narrative)
    }
}

impl NarrativeProvider for Narrative {
    fn name(&self) -> &str {
        &self.metadata.name
    }

    fn metadata(&self) -> &NarrativeMetadata {
        &self.metadata
    }

    fn act_names(&self) -> &[String] {
        &self.toc.order
    }

    fn get_act_config(&self, act_name: &str) -> Option<ActConfig> {
        self.acts.get(act_name).cloned()
    }
}
