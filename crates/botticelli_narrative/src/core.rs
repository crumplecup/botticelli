//! Core data structures for narratives.

use crate::{ActConfig, CarouselConfig, NarrativeProvider, toml_parser};
use botticelli_error::{NarrativeError, NarrativeErrorKind};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

#[cfg(feature = "database")]
use botticelli_core::Input;
#[cfg(feature = "database")]
use botticelli_database::{assemble_prompt, is_content_focus};
#[cfg(feature = "database")]
use diesel::pg::PgConnection;

/// Narrative metadata from the `[narrative]` section.
#[derive(
    Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, derive_getters::Getters,
)]
pub struct NarrativeMetadata {
    /// Unique identifier for this narrative
    name: String,
    /// Human-readable description of what this narrative does
    description: String,
    /// Optional template table to use as schema source for content generation
    template: Option<String>,
    /// Skip content generation (both template and inference modes)
    #[serde(default)]
    skip_content_generation: bool,
    /// Optional carousel configuration for narrative-level looping
    #[serde(default)]
    carousel: Option<CarouselConfig>,
    /// Optional default model for all acts in this narrative
    #[serde(default)]
    model: Option<String>,
    /// Optional default temperature for all acts in this narrative
    #[serde(default)]
    temperature: Option<f32>,
    /// Optional default max_tokens for all acts in this narrative
    #[serde(default)]
    max_tokens: Option<u32>,
}

impl NarrativeMetadata {
    /// Create a minimal test metadata (for tests only).
    #[cfg(test)]
    pub fn new_test(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: "Test narrative".to_string(),
            template: None,
            skip_content_generation: false,
            carousel: None,
            model: None,
            temperature: None,
            max_tokens: None,
        }
    }
}

/// Table of contents from the `[toc]` section.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize, derive_getters::Getters,
)]
pub struct NarrativeToc {
    /// Ordered list of act names to execute
    order: Vec<String>,
}

/// Complete narrative structure parsed from TOML.
///
/// # Example TOML Structure
///
/// Simple text acts:
/// ```toml
/// [narrative]
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, derive_getters::Getters)]
pub struct Narrative {
    /// Narrative metadata
    metadata: NarrativeMetadata,

    /// Table of contents defining execution order
    toc: NarrativeToc,

    /// Map of act names to their configurations
    #[serde(skip)]
    acts: HashMap<String, ActConfig>,

    /// Source file path (for resolving relative paths in nested narratives)
    #[serde(skip)]
    source_path: Option<std::path::PathBuf>,
}

impl Narrative {
    /// Set the source path for this narrative.
    pub fn set_source_path(&mut self, path: Option<std::path::PathBuf>) {
        self.source_path = path;
    }

    /// Loads a narrative from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML is invalid
    /// - Validation fails (missing acts, empty order, etc.)
    #[tracing::instrument(skip_all, fields(path = %path.as_ref().display()))]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, NarrativeError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| NarrativeError::new(NarrativeErrorKind::FileRead(e.to_string())))?;

        let mut narrative: Self = content.parse()?;
        narrative.source_path = Some(path.to_path_buf());
        Ok(narrative)
    }

    /// Loads a narrative from a TOML file with database-driven prompt assembly.
    ///
    /// If the narrative has a `template` field, this method will:
    /// 1. Load the narrative from the TOML file
    /// 2. For each act with a content focus (short-form prompt):
    ///    - Query the template table schema
    ///    - Generate schema documentation
    ///    - Inject platform context and format requirements
    ///    - Replace the act's prompt with the assembled version
    ///
    /// Acts with explicit full prompts (containing schema docs) are left unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML is invalid
    /// - Validation fails
    /// - Database schema reflection fails (if template specified)
    /// - Prompt assembly fails
    #[cfg(feature = "database")]
    #[tracing::instrument(skip_all, fields(path = %path.as_ref().display()))]
    pub fn from_file_with_db<P: AsRef<Path>>(
        path: P,
        conn: &mut PgConnection,
    ) -> Result<Self, NarrativeError> {
        let mut narrative = Self::from_file(path)?;
        tracing::debug!(has_template = ?narrative.metadata.template.is_some());

        // If template specified, assemble prompts with schema injection
        if narrative.metadata.template.is_some() {
            narrative.assemble_act_prompts(conn)?;
        }

        Ok(narrative)
    }

    /// Assembles prompts for all acts using template schema injection.
    ///
    /// For each act:
    /// - Extracts the first text input (assumes simple text prompts)
    /// - Checks if it's a content focus (short-form) or explicit prompt
    /// - If content focus, assembles complete prompt with schema + format requirements
    /// - If explicit, leaves unchanged (backward compatibility)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Template field is not set
    /// - Database schema reflection fails
    /// - Prompt assembly fails
    #[cfg(feature = "database")]
    #[tracing::instrument(skip(self, conn), fields(template = ?self.metadata.template, act_count = self.acts.len()))]
    pub fn assemble_act_prompts(&mut self, conn: &mut PgConnection) -> Result<(), NarrativeError> {
        let template = self
            .metadata
            .template
            .as_ref()
            .ok_or_else(|| NarrativeError::new(NarrativeErrorKind::MissingTemplate))?;

        for (act_name, config) in &mut self.acts {
            // Get the first text input (most common case)
            if let Some(Input::Text(user_prompt)) = config.inputs().first() {
                // Check if this is a content focus or explicit prompt
                if is_content_focus(user_prompt) {
                    // Assemble complete prompt with schema injection
                    let assembled = assemble_prompt(conn, template, user_prompt).map_err(|e| {
                        NarrativeError::new(NarrativeErrorKind::PromptAssembly {
                            act: act_name.clone(),
                            message: e.to_string(),
                        })
                    })?;

                    // Replace the first input with assembled prompt
                    let mut new_inputs = config.inputs().clone();
                    new_inputs[0] = Input::Text(assembled);
                    *config = config.clone().with_inputs(new_inputs);

                    tracing::debug!(
                        act = %act_name,
                        template = %template,
                        "Assembled prompt with schema injection"
                    );
                } else {
                    tracing::debug!(
                        act = %act_name,
                        "Skipping prompt assembly (explicit full prompt detected)"
                    );
                }
            }
        }

        Ok(())
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
    #[tracing::instrument(skip(self), fields(name = %self.metadata.name, act_count = self.toc.order.len()))]
    pub fn validate(&self) -> Result<(), NarrativeError> {
        // Check that toc.order is not empty
        if self.toc.order.is_empty() {
            return Err(NarrativeError::new(NarrativeErrorKind::EmptyToc));
        }

        // Check that all acts in toc.order exist in acts map
        for act_name in &self.toc.order {
            if !self.acts.contains_key(act_name) {
                return Err(NarrativeError::new(NarrativeErrorKind::MissingAct(
                    act_name.clone(),
                )));
            }
        }

        // Check that all acts have at least one input
        for (act_name, config) in &self.acts {
            if config.inputs().is_empty() {
                return Err(NarrativeError::new(NarrativeErrorKind::EmptyPrompt(
                    act_name.clone(),
                )));
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
            .filter_map(|name| self.acts.get(name).map(|config| (name.as_str(), config)))
            .collect()
    }
}

impl FromStr for Narrative {
    type Err = NarrativeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_toml_str(s, None)
    }
}

impl Narrative {
    /// Parse a narrative from TOML string, optionally specifying which narrative to load.
    pub fn from_toml_str(s: &str, narrative_name: Option<&str>) -> Result<Self, NarrativeError> {
        // Parse TOML into intermediate structure
        let toml_narrative_file: toml_parser::TomlNarrativeFile = toml::from_str(s)
            .map_err(|e| NarrativeError::new(NarrativeErrorKind::TomlParse(e.to_string())))?;

        // Resolve the narrative (supports multi-narrative files)
        let (narrative_meta, narrative_toc, narrative_acts) = toml_narrative_file
            .resolve_narrative(narrative_name)
            .map_err(|e| NarrativeError::new(NarrativeErrorKind::TomlParse(e)))?;

        // Convert to domain types
        let metadata = NarrativeMetadata {
            name: narrative_meta.name.clone(),
            description: narrative_meta.description.clone(),
            template: narrative_meta.template.clone(),
            skip_content_generation: narrative_meta.skip_content_generation,
            carousel: narrative_meta.carousel.clone(),
            model: narrative_meta.model.clone(),
            temperature: narrative_meta.temperature,
            max_tokens: narrative_meta.max_tokens,
        };

        let toc = NarrativeToc {
            order: narrative_toc.order.clone(),
        };

        let mut acts = HashMap::new();
        for (act_name, toml_act) in &narrative_acts {
            let act_config = toml_act.to_act_config(&toml_narrative_file).map_err(|e| {
                // Check if this is an empty prompt error
                if e.contains("empty") || e.contains("whitespace") {
                    NarrativeError::new(NarrativeErrorKind::EmptyPrompt(act_name.clone()))
                } else {
                    NarrativeError::new(NarrativeErrorKind::TomlParse(format!(
                        "Act '{}': {}",
                        act_name, e
                    )))
                }
            })?;
            acts.insert(act_name.clone(), act_config);
        }

        let narrative = Narrative {
            metadata,
            toc,
            acts,
            source_path: None,
        };
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

    fn carousel_config(&self) -> Option<&CarouselConfig> {
        self.metadata.carousel.as_ref()
    }

    fn source_path(&self) -> Option<&std::path::Path> {
        self.source_path.as_deref()
    }
}
