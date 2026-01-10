//! TOML file structure and reference resolution.

use super::{
    definitions::*, narrative::*, utils::*, TomlAct, TomlNarrativeDefinition,
    TomlNarrativeReference,
};
use botticelli_core::{HistoryRetention, Input, MediaSource};
use botticelli_error::{NarrativeErrorKind, NarrativeResult};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, error, instrument};

/// Root TOML structure supporting both single and multi-narrative files.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlNarrativeFile {
    /// Shared act definitions (available to all narratives)
    #[serde(default)]
    acts: HashMap<String, TomlAct>,

    /// Optional bot command definitions (shared)
    #[serde(default)]
    bots: HashMap<String, TomlBotDefinition>,

    /// Optional table query definitions (shared)
    #[serde(default)]
    tables: HashMap<String, TomlTableDefinition>,

    /// Optional media source definitions (shared)
    #[serde(default)]
    media: HashMap<String, TomlMediaDefinition>,

    /// Flattened narrative field - can be single TomlNarrative or HashMap<String, TomlNarrativeEntry>
    #[serde(flatten)]
    narrative_data: TomlNarrativeData,
}

/// Wrapper for either single narrative or multi-narrative format
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlNarrativeData {
    /// Single narrative with optional TOC at root
    Single {
        #[serde(default)]
        narrative: Box<Option<TomlNarrative>>,
        #[serde(default)]
        toc: Option<TomlToc>,
    },
    /// Multi-narrative with [narrative.name] entries
    Multi {
        #[serde(default)]
        narrative: HashMap<String, TomlNarrativeEntry>,
    },
}

/// Entry in [narratives.name] can be either a reference or inline definition.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlNarrativeEntry {
    /// Reference to another narrative file
    Reference(TomlNarrativeReference),
    /// Inline narrative definition
    Definition(Box<TomlNarrativeDefinition>),
}

impl TomlNarrativeFile {
    /// Check if this file contains multiple narratives.
    #[instrument(skip(self))]
    pub fn is_multi_narrative(&self) -> bool {
        let result = match &self.narrative_data {
            TomlNarrativeData::Single { .. } => false,
            TomlNarrativeData::Multi { .. } => true,
        };
        debug!(is_multi = result, "Checked narrative file type");
        result
    }

    /// Resolve a narrative from this file.
    ///
    /// If `narrative_name` is provided, looks for it in [narrative.name].
    /// Otherwise, uses the single [narrative] section if present.
    ///
    /// Returns a tuple of (metadata fields, toc, acts map).
    #[instrument(skip(self), fields(narrative_name = ?narrative_name))]
    pub fn resolve_narrative(
        &self,
        narrative_name: Option<&str>,
    ) -> NarrativeResult<(TomlNarrative, TomlToc, HashMap<String, TomlAct>)> {
        debug!("Resolving narrative from file");
        // Extract the narrative map from narrative_data
        let (single_narrative, single_toc, multi_map) = match &self.narrative_data {
            TomlNarrativeData::Single { narrative, toc } => {
                debug!("Processing single narrative file");
                (narrative.as_ref().as_ref(), toc.as_ref(), None)
            }
            TomlNarrativeData::Multi { narrative } => {
                debug!(narrative_count = narrative.len(), "Processing multi-narrative file");
                (None, None, Some(narrative))
            }
        };

        match (narrative_name, single_narrative, multi_map) {
            // Explicit name provided - look in [narrative.name]
            (Some(name), _, Some(multi)) if !multi.is_empty() => {
                debug!(%name, "Looking up narrative by name");
                let entry = multi.get(name).ok_or_else(|| {
                    let available: Vec<_> = multi.keys().cloned().collect();
                    error!(%name, available = %available.join(", "), "Narrative not found");
                    NarrativeErrorKind::NarrativeNotFound {
                        name: name.to_string(),
                        available: available.join(", "),
                    }
                })?;

                self.process_narrative_entry(name, entry)
            }

            // No name provided, use single [narrative]
            (None, Some(single), _) => {
                debug!("Using single narrative");
                let toc = single_toc.ok_or_else(|| {
                    error!("Single narrative missing toc field");
                    NarrativeErrorKind::MissingRequiredField {
                        field: "toc".to_string(),
                        input_type: "narrative".to_string(),
                    }
                })?;
                debug!(act_count = self.acts.len(), toc_length = toc.order().len(), "Resolved single narrative");
                Ok((single.clone(), toc.clone(), self.acts.clone()))
            }

            // No name, multiple definitions exist - ambiguous
            (None, _, Some(multi)) if !multi.is_empty() => {
                let available: Vec<_> = multi.keys().cloned().collect();
                error!(available = %available.join(", "), "Multiple narratives found but no name provided");
                Err(NarrativeErrorKind::AmbiguousNarrative {
                    available: available.join(", "),
                }
                .into())
            }

            // No narrative found at all
            _ => {
                error!("No narrative found in file");
                Err(NarrativeErrorKind::NoNarrativeFound.into())
            }
        }
    }

    #[instrument(skip(self, entry), fields(%name))]
    fn process_narrative_entry(
        &self,
        name: &str,
        entry: &TomlNarrativeEntry,
    ) -> NarrativeResult<(TomlNarrative, TomlToc, HashMap<String, TomlAct>)> {
        debug!("Processing narrative entry");
        match entry {
            TomlNarrativeEntry::Definition(def) => {
                // Use table key as name if not specified inline
                let narrative_name = def.name.clone().unwrap_or_else(|| name.to_string());
                let narrative_desc = def.description.clone().unwrap_or_default();

                // Convert definition to TomlNarrative format
                let meta = TomlNarrative {
                    name: narrative_name,
                    description: narrative_desc,
                    template: def.template.clone(),
                    target: def.target.clone(),
                    skip_content_generation: def.skip_content_generation,
                    carousel: def.carousel.clone(),
                    model: def.model.clone(),
                    temperature: def.temperature,
                    max_tokens: def.max_tokens,
                    budget: def.budget.clone(),
                };

                // Merge shared acts with definition-specific acts
                let mut acts = self.acts.clone();
                acts.extend(def.acts.clone()); // Definition acts override shared

                // Convert toc Vec to TomlToc::Array
                let toc = TomlToc::Array(def.toc.clone());
                debug!(act_count = acts.len(), toc_length = toc.order().len(), "Narrative entry processed");
                Ok((meta, toc, acts))
            }
            TomlNarrativeEntry::Reference(_) => {
                error!("Cannot use file reference as inline definition");
                Err(NarrativeErrorKind::WrongReferenceType {
                    reference: name.to_string(),
                    found: "file reference".to_string(),
                    expected: "inline definition".to_string(),
                }
                .into())
            }
        }
    }

    /// Resolve a resource reference to an Input.
    #[instrument(skip(self), fields(reference))]
    pub fn resolve_reference(&self, reference: &str) -> NarrativeResult<Input> {
        debug!(%reference, "Resolving resource reference");

        // Handle narrative: prefix specially
        if let Some(narrative_name) = reference.strip_prefix("narrative:") {
            debug!(narrative_name, "Resolved narrative reference");
            return Ok(Input::Narrative {
                name: narrative_name.to_string(),
                path: None, // Will be resolved relative to calling narrative
                history_retention: HistoryRetention::Full,
            });
        }

        let parts: Vec<&str> = reference.split('.').collect();
        if parts.len() != 2 {
            error!(%reference, "Invalid reference format (expected 'category.name')");
            return Err(NarrativeErrorKind::InvalidReferenceFormat {
                reference: reference.to_string(),
                reason: "expected 'category.name' format".to_string(),
            }
            .into());
        }

        let (category, name) = (parts[0], parts[1]);
        debug!(category, name, "Parsed reference");

        match category {
            "bots" => self.resolve_bot_reference(name),
            "tables" => self.resolve_table_reference(name),
            "media" => self.resolve_media_reference(name),
            "narratives" => self.resolve_narrative_reference(name),
            _ => {
                error!(category, "Unknown reference category");
                Err(NarrativeErrorKind::InvalidReferenceFormat {
                    reference: reference.to_string(),
                    reason: format!("unknown category '{}', must be 'bots', 'tables', 'media', or 'narratives'", category),
                }
                .into())
            }
        }
    }

    #[instrument(skip(self), fields(name))]
    fn resolve_bot_reference(&self, name: &str) -> NarrativeResult<Input> {
        debug!(%name, "Resolving bot reference");
        let bot_def = self.bots.get(name).ok_or_else(|| {
            error!(%name, "Bot not found");
            NarrativeErrorKind::ResourceNotFound {
                resource_type: "bot".to_string(),
                name: name.to_string(),
            }
        })?;

        debug!(platform = %bot_def.platform(), command = %bot_def.command(), "Bot reference resolved");
        let args = expand_env_vars(bot_def.args());
        Ok(Input::BotCommand {
            platform: bot_def.platform().clone(),
            command: bot_def.command().clone(),
            args,
            required: false,
            cache_duration: None,
            history_retention: HistoryRetention::Full,
        })
    }

    #[instrument(skip(self), fields(name))]
    fn resolve_table_reference(&self, name: &str) -> NarrativeResult<Input> {
        debug!(%name, "Resolving table reference");
        let table_def = self.tables.get(name).ok_or_else(|| {
            error!(%name, "Table not found");
            NarrativeErrorKind::ResourceNotFound {
                resource_type: "table".to_string(),
                name: name.to_string(),
            }
        })?;

        use botticelli_core::TableFormat;
        let format = match table_def.format().as_deref() {
            Some("json") | None => TableFormat::Json,
            Some("markdown") => TableFormat::Markdown,
            Some("csv") => TableFormat::Csv,
            Some(f) => {
                error!(format = f, "Unknown table format");
                return Err(NarrativeErrorKind::InvalidFieldValue {
                    value: f.to_string(),
                    field: "format".to_string(),
                    reason: "must be 'json', 'markdown', or 'csv'".to_string(),
                }
                .into());
            }
        };

        debug!(
            table_name = %table_def.table_name(),
            ?format,
            limit = ?table_def.limit(),
            "Table reference resolved"
        );
        Ok(Input::Table {
            table_name: table_def.table_name().clone(),
            columns: table_def.columns().clone(),
            where_clause: table_def.where_clause().clone(),
            limit: *table_def.limit(),
            offset: *table_def.offset(),
            order_by: table_def.order_by().clone(),
            alias: Some(name.to_string()),
            format,
            sample: *table_def.sample(),
            destructive_read: table_def.pull_and_delete().unwrap_or(true),
            history_retention: HistoryRetention::Full,
        })
    }

    #[instrument(skip(self), fields(name))]
    fn resolve_media_reference(&self, name: &str) -> NarrativeResult<Input> {
        debug!(%name, "Resolving media reference");
        let media_def = self.media.get(name).ok_or_else(|| {
            error!(%name, "Media not found");
            NarrativeErrorKind::ResourceNotFound {
                resource_type: "media".to_string(),
                name: name.to_string(),
            }
        })?;

        // Detect media source
        let source = if let Some(url) = media_def.url() {
            debug!(%url, "Using URL source");
            MediaSource::Url(url.clone())
        } else if let Some(file) = media_def.file() {
            debug!(%file, "Reading file source");
            let data = std::fs::read(file).map_err(|e| {
                error!(%file, error = %e, "Failed to read file");
                NarrativeErrorKind::FileRead(format!("Failed to read file {}: {}", file, e))
            })?;
            debug!(%file, size = data.len(), "File read successfully");
            MediaSource::Binary(data)
        } else if let Some(base64) = media_def.base64() {
            debug!(base64_len = base64.len(), "Using base64 source");
            MediaSource::Base64(base64.clone())
        } else {
            error!(%name, "Media definition missing source (url, file, or base64)");
            return Err(NarrativeErrorKind::MissingRequiredField {
                field: "url, file, or base64".to_string(),
                input_type: format!("media[{}]", name),
            }
            .into());
        };

        // Infer MIME type if not provided
        let mime = media_def.mime().clone().or_else(|| {
            media_def
                .file()
                .as_ref()
                .or(media_def.url().as_ref())
                .and_then(|path| infer_mime_type(path))
        });

        // Infer media type from MIME or extension
        let media_type = if let Some(mime_str) = &mime {
            match mime_str.split('/').next() {
                Some("image") => "image",
                Some("audio") => "audio",
                Some("video") => "video",
                Some("application") | Some("text") => "document",
                _ => {
                    error!(mime = %mime_str, "Cannot determine media type from MIME");
                    return Err(NarrativeErrorKind::InvalidFieldValue {
                        value: mime_str.clone(),
                        field: "mime".to_string(),
                        reason: "unsupported MIME type, must start with image/, audio/, video/, application/, or text/".to_string(),
                    }
                    .into());
                }
            }
        } else {
            // Infer from file extension
            let path = media_def
                .file()
                .as_ref()
                .or(media_def.url().as_ref())
                .ok_or_else(|| {
                    error!(%name, "Cannot infer media type without file path or MIME");
                    NarrativeErrorKind::MissingRequiredField {
                        field: "mime or file path".to_string(),
                        input_type: format!("media[{}]", name),
                    }
                })?;
            infer_media_type_from_extension(path)?
        };

        debug!(media_type, ?mime, "Media reference resolved");
        match media_type {
            "image" => Ok(Input::Image { mime, source }),
            "audio" => Ok(Input::Audio { mime, source }),
            "video" => Ok(Input::Video { mime, source }),
            "document" => Ok(Input::Document {
                mime,
                source,
                filename: media_def.filename().clone(),
            }),
            _ => {
                error!(media_type, "Unknown media type");
                Err(NarrativeErrorKind::InvalidFieldValue {
                    value: media_type.to_string(),
                    field: "type".to_string(),
                    reason: "must be 'image', 'audio', 'video', or 'document'".to_string(),
                }
                .into())
            }
        }
    }

    #[instrument(skip(self), fields(name))]
    fn resolve_narrative_reference(&self, name: &str) -> NarrativeResult<Input> {
        debug!(%name, "Resolving narrative reference");
        
        // Get the multi-narrative map if it exists
        let multi_map = match &self.narrative_data {
            TomlNarrativeData::Multi { narrative } => narrative,
            TomlNarrativeData::Single { .. } => {
                error!("Cannot reference narratives from single-narrative file");
                return Err(NarrativeErrorKind::ResourceNotFound {
                    resource_type: "narrative".to_string(),
                    name: name.to_string(),
                }
                .into());
            }
        };
        
        let narrative_entry = multi_map.get(name).ok_or_else(|| {
            error!(%name, "Narrative not found");
            NarrativeErrorKind::ResourceNotFound {
                resource_type: "narrative".to_string(),
                name: name.to_string(),
            }
        })?;

        match narrative_entry {
            TomlNarrativeEntry::Reference(ref_def) => {
                debug!(narrative_file = %ref_def.narrative(), "Narrative file reference resolved");
                Ok(Input::Narrative {
                    name: name.to_string(),
                    path: Some(ref_def.narrative().clone()),
                    history_retention: HistoryRetention::Full,
                })
            }
            TomlNarrativeEntry::Definition(_) => {
                error!(%name, "Cannot use inline narrative definition as input");
                Err(NarrativeErrorKind::WrongReferenceType {
                    reference: name.to_string(),
                    found: "inline definition".to_string(),
                    expected: "file reference".to_string(),
                }
                .into())
            }
        }
    }
}
