//! Multi-narrative container for narrative composition.
//!
//! This module provides `MultiNarrative`, which loads all narratives from a TOML file
//! and enables narrative composition (narratives referencing other narratives).

use crate::{ActConfig, CarouselConfig, Narrative, NarrativeMetadata, NarrativeProvider};
use botticelli_error::{NarrativeError, NarrativeErrorKind};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, instrument};

#[cfg(feature = "database")]
use diesel::pg::PgConnection;

/// Container for multiple narratives from a single TOML file.
///
/// Enables narrative composition where narratives can reference each other.
#[derive(Debug, Clone)]
pub struct MultiNarrative {
    narratives: HashMap<String, Narrative>,
    active_narrative: String,
}

impl MultiNarrative {
    /// Load all narratives from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML file
    /// * `narrative_name` - Name of the narrative to set as active
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    #[instrument(skip_all, fields(path = %path.as_ref().display(), narrative_name))]
    pub fn from_file<P: AsRef<Path>>(
        path: P,
        narrative_name: &str,
    ) -> Result<Self, NarrativeError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| NarrativeError::new(NarrativeErrorKind::FileRead(e.to_string())))?;

        Self::from_toml_str(&content, path, narrative_name)
    }

    /// Load all narratives from a TOML file with database support.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML file
    /// * `narrative_name` - Name of the narrative to set as active
    /// * `conn` - Database connection for schema reflection
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or schema reflection fails.
    #[cfg(feature = "database")]
    #[instrument(skip_all, fields(path = %path.as_ref().display(), narrative_name))]
    pub fn from_file_with_db<P: AsRef<Path>>(
        path: P,
        narrative_name: &str,
        conn: &mut PgConnection,
    ) -> Result<Self, NarrativeError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| NarrativeError::new(NarrativeErrorKind::FileRead(e.to_string())))?;

        Self::from_toml_str_with_db(&content, path, narrative_name, conn)
    }

    /// Parse all narratives from TOML string.
    #[instrument(skip_all, fields(narrative_name))]
    fn from_toml_str(
        s: &str,
        source_path: &Path,
        narrative_name: &str,
    ) -> Result<Self, NarrativeError> {
        use crate::toml_parser::{TomlNarrativeData, TomlNarrativeFile};

        // Parse the TOML file
        let toml_file: TomlNarrativeFile = toml::from_str(s)
            .map_err(|e| NarrativeError::new(NarrativeErrorKind::TomlParse(e.to_string())))?;

        // Extract all narrative names from [narrative.name] or [narratives.name]
        let narrative_names: Vec<String> = match &toml_file.narrative_data {
            TomlNarrativeData::Multi { narrative } => narrative.keys().cloned().collect(),
            TomlNarrativeData::Single { narrative, .. } => {
                // Single narrative - get its name
                if let Some(n) = narrative.as_ref() {
                    vec![n.name().clone()]
                } else {
                    vec![]
                }
            }
        };

        debug!(count = narrative_names.len(), names = ?narrative_names, "Found narratives in file");

        // Load each narrative
        let mut narratives = HashMap::new();
        for name in &narrative_names {
            let mut narrative = Narrative::from_toml_str(s, Some(name))?;
            narrative.set_source_path(Some(source_path.to_path_buf()));
            narratives.insert(name.clone(), narrative);
        }

        // Verify the requested narrative exists
        if !narratives.contains_key(narrative_name) {
            return Err(NarrativeError::new(NarrativeErrorKind::TomlParse(format!(
                "Narrative '{}' not found. Available: {}",
                narrative_name,
                narrative_names.join(", ")
            ))));
        }

        Ok(Self {
            narratives,
            active_narrative: narrative_name.to_string(),
        })
    }

    /// Parse all narratives from TOML string with database support.
    #[cfg(feature = "database")]
    #[instrument(skip_all, fields(narrative_name))]
    fn from_toml_str_with_db(
        s: &str,
        source_path: &Path,
        narrative_name: &str,
        conn: &mut PgConnection,
    ) -> Result<Self, NarrativeError> {
        use crate::toml_parser::{TomlNarrativeData, TomlNarrativeFile};

        // Parse the TOML file
        let toml_file: TomlNarrativeFile = toml::from_str(s)
            .map_err(|e| NarrativeError::new(NarrativeErrorKind::TomlParse(e.to_string())))?;

        // Extract all narrative names from [narrative.name] or [narratives.name]
        let narrative_names: Vec<String> = match &toml_file.narrative_data {
            TomlNarrativeData::Multi { narrative } => narrative.keys().cloned().collect(),
            TomlNarrativeData::Single { narrative, .. } => {
                if let Some(n) = narrative.as_ref() {
                    vec![n.name().clone()]
                } else {
                    vec![]
                }
            }
        };

        debug!(count = narrative_names.len(), names = ?narrative_names, "Found narratives in file");

        // Load each narrative with database support
        let mut narratives = HashMap::new();
        for name in &narrative_names {
            // Parse narrative from TOML
            let mut narrative = Narrative::from_toml_str(s, Some(name))?;
            narrative.set_source_path(Some(source_path.to_path_buf()));

            // Assemble prompts if template specified
            if narrative.metadata().template().is_some() {
                narrative.assemble_act_prompts(conn)?;
            }

            narratives.insert(name.clone(), narrative);
        }

        // Verify the requested narrative exists
        if !narratives.contains_key(narrative_name) {
            return Err(NarrativeError::new(NarrativeErrorKind::TomlParse(format!(
                "Narrative '{}' not found. Available: {}",
                narrative_name,
                narrative_names.join(", ")
            ))));
        }

        Ok(Self {
            narratives,
            active_narrative: narrative_name.to_string(),
        })
    }

    /// Get a narrative by name for composition.
    pub fn get_narrative(&self, name: &str) -> Option<&Narrative> {
        self.narratives.get(name)
    }
}

impl NarrativeProvider for MultiNarrative {
    fn name(&self) -> &str {
        &self.active_narrative
    }

    fn metadata(&self) -> &NarrativeMetadata {
        self.narratives
            .get(&self.active_narrative)
            .map(|n| n.metadata())
            .unwrap_or_else(|| panic!("Active narrative {} must exist", self.active_narrative))
    }

    fn act_names(&self) -> &[String] {
        self.narratives
            .get(&self.active_narrative)
            .map(|n| n.act_names())
            .unwrap_or_else(|| panic!("Active narrative {} must exist", self.active_narrative))
    }

    fn get_act_config(&self, act_name: &str) -> Option<ActConfig> {
        self.narratives
            .get(&self.active_narrative)
            .and_then(|n| n.get_act_config(act_name))
    }

    fn carousel_config(&self) -> Option<&CarouselConfig> {
        self.narratives
            .get(&self.active_narrative)
            .and_then(|n| n.carousel_config())
    }

    fn source_path(&self) -> Option<&Path> {
        self.narratives
            .get(&self.active_narrative)
            .and_then(|n| NarrativeProvider::source_path(n))
    }

    fn resolve_narrative(&self, narrative_name: &str) -> Option<&dyn NarrativeProvider> {
        self.narratives
            .get(narrative_name)
            .map(|n| n as &dyn NarrativeProvider)
    }
}
