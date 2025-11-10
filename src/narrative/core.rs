//! Core data structures for narratives.

use crate::narrative::{ActConfig, NarrativeError, NarrativeErrorKind, NarrativeProvider};
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Narrative {
    /// Narrative metadata
    #[serde(rename = "narration")]
    pub metadata: NarrativeMetadata,

    /// Table of contents defining execution order
    pub toc: NarrativeToc,

    /// Map of act names to their prompts
    pub acts: HashMap<String, String>,
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
    /// - All act prompts are non-empty
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

        // Check that all act prompts are non-empty
        for (act_name, prompt) in &self.acts {
            if prompt.trim().is_empty() {
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
    /// Each tuple contains the act name and its prompt.
    pub fn ordered_acts(&self) -> Vec<(&str, &str)> {
        self.toc
            .order
            .iter()
            .filter_map(|name| {
                self.acts
                    .get(name)
                    .map(|prompt| (name.as_str(), prompt.as_str()))
            })
            .collect()
    }
}

impl FromStr for Narrative {
    type Err = NarrativeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let narrative: Narrative = toml::from_str(s).map_err(|e| {
            NarrativeError::new(NarrativeErrorKind::TomlParse(e.to_string()), line!(), file!())
        })?;

        narrative.validate()?;
        Ok(narrative)
    }
}

impl NarrativeProvider for Narrative {
    fn name(&self) -> &str {
        &self.metadata.name
    }

    fn act_names(&self) -> &[String] {
        &self.toc.order
    }

    fn get_act_config(&self, act_name: &str) -> Option<ActConfig> {
        self.acts
            .get(act_name)
            .map(|prompt| ActConfig::from_text(prompt.clone()))
    }
}
