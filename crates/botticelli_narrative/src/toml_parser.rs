//! TOML deserialization structures for narrative configuration.
//!
//! This module provides intermediate structures for deserializing TOML
//! into our domain types (ActConfig, Input, etc.).

use crate::ActConfig;
use botticelli_core::{HistoryRetention, Input, MediaSource};
use botticelli_error::{IoError, NarrativeError, NarrativeErrorKind, NarrativeResult};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, error, instrument, warn};

/// Recursively search for a file starting from a base directory.
///
/// Searches upward through parent directories until the file is found or root is reached.
/// Then searches downward recursively from the highest found directory.
///
/// # Arguments
///
/// * `filename` - The filename to search for (without path, e.g., "BOTTICELLI_CONTEXT.md")
/// * `start_dir` - Optional starting directory (defaults to current directory)
/// * `context_path` - Optional context base path from configuration
///
/// # Returns
///
/// The full path to the file if found, or an error if not found.
#[instrument(skip_all, fields(%filename))]
fn find_file_recursive(
    filename: &str,
    start_dir: Option<&Path>,
    context_path: Option<&str>,
) -> NarrativeResult<PathBuf> {
    // Determine search start directory
    let base_dir = if let Some(ctx_path) = context_path {
        PathBuf::from(ctx_path)
    } else if let Some(start) = start_dir {
        start.to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| {
            NarrativeError::new(NarrativeErrorKind::Io(IoError::from(e)))
        })?
    };

    debug!(search_base = %base_dir.display(), "Starting file search");

    // First try exact match in base directory
    let direct_path = base_dir.join(filename);
    if direct_path.exists() {
        debug!(found = %direct_path.display(), "Found file directly");
        return Ok(direct_path);
    }

    // Search upward to find project root or file
    let mut current = base_dir.as_path();
    let mut search_roots = vec![current.to_path_buf()];

    while let Some(parent) = current.parent() {
        let candidate = parent.join(filename);
        if candidate.exists() {
            debug!(found = %candidate.display(), "Found file in parent directory");
            return Ok(candidate);
        }

        // Check for workspace markers (stop at project root)
        if parent.join("Cargo.toml").exists() || parent.join(".git").exists() {
            search_roots.push(parent.to_path_buf());
            break;
        }

        search_roots.push(parent.to_path_buf());
        current = parent;
    }

    // Search downward recursively from collected roots
    for root in search_roots.iter().rev() {
        if let Ok(found) = search_directory_recursive(root, filename) {
            debug!(found = %found.display(), "Found file via recursive search");
            return Ok(found);
        }
    }

    error!("File not found after exhaustive search");
    Err(NarrativeErrorKind::FileRead(format!(
        "File '{}' not found in {} or any parent/child directories",
        filename,
        base_dir.display()
    ))
    .into())
}

/// Recursively search a directory tree for a file.
fn search_directory_recursive(dir: &Path, filename: &str) -> Result<PathBuf, ()> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            // Check if this is the file we're looking for
            if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(filename) {
                return Ok(path);
            }

            // Recurse into subdirectories (skip hidden dirs and common ignore patterns)
            if path.is_dir()
                && let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
                && !dir_name.starts_with('.')
                && dir_name != "target"
                && dir_name != "node_modules"
                && let Ok(found) = search_directory_recursive(&path, filename)
            {
                return Ok(found);
            }
        }
    }

    Err(())
}

/// Expand environment variables in string values within a HashMap.
///
/// Supports `${VAR_NAME}` and `$VAR_NAME` syntax.
fn expand_env_vars(
    args: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    args.iter()
        .map(|(k, v)| {
            let expanded_value = match v {
                serde_json::Value::String(s) => {
                    match shellexpand::env(s) {
                        Ok(expanded) => serde_json::Value::String(expanded.into_owned()),
                        Err(_) => v.clone(), // Keep original if expansion fails
                    }
                }
                _ => v.clone(),
            };
            (k.clone(), expanded_value)
        })
        .collect()
}

/// Intermediate structure for deserializing the [narrative] section (single narrative).
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlNarrative {
    name: String,
    description: String,
    /// Optional template table to use as schema source for content generation
    template: Option<String>,
    /// Optional target table name for content generation (overrides narrative name)
    target: Option<String>,
    /// Optional flag to skip content generation (both template and inference modes)
    #[serde(default)]
    skip_content_generation: bool,
    /// Optional carousel configuration
    #[serde(default)]
    carousel: Option<crate::CarouselConfig>,
    /// Optional default model for all acts
    #[serde(default)]
    model: Option<String>,
    /// Optional default temperature for all acts
    #[serde(default)]
    temperature: Option<f32>,
    /// Optional default max_tokens for all acts
    #[serde(default)]
    max_tokens: Option<u32>,
    /// Optional budget multipliers
    #[serde(default)]
    budget: Option<botticelli_core::BudgetConfig>,
}

/// Intermediate structure for deserializing individual [narratives.name] sections.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlNarrativeDefinition {
    /// Name is optional here because it comes from the table key [narratives.NAME]
    #[serde(default)]
    name: Option<String>,
    /// Description is now optional
    #[serde(default)]
    description: Option<String>,
    /// Optional template table to use as schema source for content generation
    template: Option<String>,
    /// Optional target table name for content generation (overrides narrative name)
    target: Option<String>,
    /// Optional flag to skip content generation
    #[serde(default)]
    skip_content_generation: bool,
    /// Optional carousel configuration
    #[serde(default)]
    carousel: Option<crate::CarouselConfig>,
    /// Optional default model
    #[serde(default)]
    model: Option<String>,
    /// Optional default temperature
    #[serde(default)]
    temperature: Option<f32>,
    /// Optional default max_tokens
    #[serde(default)]
    max_tokens: Option<u32>,
    /// Optional budget multipliers
    #[serde(default)]
    budget: Option<botticelli_core::BudgetConfig>,
    /// Table of contents for this narrative (just an array of act names)
    toc: Vec<String>,
    /// Optional narrative-specific acts (override shared acts)
    #[serde(default)]
    acts: HashMap<String, TomlAct>,
}

/// Intermediate structure for deserializing the [toc] section (backwards compat).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlToc {
    /// Simple array: toc = ["one", "two"]
    Array(Vec<String>),
    /// Structured: [toc] with order field
    Structured { order: Vec<String> },
}

impl TomlToc {
    /// Get the order vector regardless of variant.
    pub fn order(&self) -> &[String] {
        match self {
            TomlToc::Array(v) => v,
            TomlToc::Structured { order } => order,
        }
    }
}

/// Bot command definition from [bots.name] section.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlBotDefinition {
    platform: String,
    command: String,
    /// All other fields are flattened into args
    #[serde(flatten)]
    args: HashMap<String, serde_json::Value>,
}

/// Table query definition from [tables.name] section.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlTableDefinition {
    table_name: String,
    columns: Option<Vec<String>>,
    #[serde(rename = "where")]
    where_clause: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    order_by: Option<String>,
    format: Option<String>,
    sample: Option<u32>,
    pull_and_delete: Option<bool>,
}

/// Media source definition from [media.name] section.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlMediaDefinition {
    url: Option<String>,
    file: Option<String>,
    base64: Option<String>,
    mime: Option<String>,
    filename: Option<String>,
}

/// Nested narrative reference from [narratives.name] section.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlNarrativeReference {
    narrative: String,
}

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
    input: Vec<TomlInput>,

    /// Reference to another narrative to execute as this act
    #[serde(default, alias = "narrative_ref")]
    narrative: Option<String>,

    /// Optional model override
    model: Option<String>,

    /// Optional temperature override
    temperature: Option<f32>,

    /// Optional max_tokens override
    max_tokens: Option<u32>,

    /// Optional carousel configuration for this act
    #[serde(default)]
    carousel: Option<crate::CarouselConfig>,

    /// Whether to extract and store JSON output (default: only for last act in narrative)
    #[serde(default)]
    extract_output: Option<bool>,
}

/// TOML representation of an input.
///
/// The `type` field determines which other fields are required.
/// Source is detected from which of url/base64/file is present.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct TomlInput {
    /// Input type: "text", "image", "audio", "video", "document", "bot_command", "table"
    #[serde(rename = "type")]
    input_type: Option<String>,

    /// Reference to a resource: "bots.name", "tables.name", "media.name"
    #[serde(rename = "ref")]
    reference: Option<String>,

    // Text input field
    content: Option<String>,

    // Media input fields
    mime: Option<String>,
    url: Option<String>,
    base64: Option<String>,
    file: Option<String>,

    // Document-specific field
    filename: Option<String>,

    // Bot command fields
    platform: Option<String>,
    command: Option<String>,
    args: Option<HashMap<String, serde_json::Value>>,
    required: Option<bool>,
    cache_duration: Option<u64>,

    // Table reference fields
    table_name: Option<String>,
    columns: Option<Vec<String>>,
    #[serde(rename = "where")]
    where_clause: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    order_by: Option<String>,
    format: Option<String>,
    sample: Option<u32>,

    // History retention field (applies to bot_command, table, narrative)
    history_retention: Option<String>,

    // Pull and delete flag for destructive reads (table only)
    pull_and_delete: Option<bool>,
}

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

    /// Legacy support for [narratives.name] syntax (deprecated)
    #[serde(default)]
    narratives: HashMap<String, TomlNarrativeEntry>,
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
    pub fn is_multi_narrative(&self) -> bool {
        match &self.narrative_data {
            TomlNarrativeData::Single { .. } => false,
            TomlNarrativeData::Multi { .. } => true,
        }
    }

    /// Resolve a narrative from this file.
    ///
    /// If `narrative_name` is provided, looks for it in [narrative.name] or [narratives.name].
    /// Otherwise, uses the single [narrative] section if present.
    ///
    /// Returns a tuple of (metadata fields, toc, acts map, shared resources).
    pub fn resolve_narrative(
        &self,
        narrative_name: Option<&str>,
    ) -> NarrativeResult<(TomlNarrative, TomlToc, HashMap<String, TomlAct>)> {
        // Extract the narrative map from narrative_data
        let (single_narrative, single_toc, multi_map) = match &self.narrative_data {
            TomlNarrativeData::Single { narrative, toc } => {
                (narrative.as_ref().as_ref(), toc.as_ref(), None)
            }
            TomlNarrativeData::Multi { narrative } => (None, None, Some(narrative)),
        };

        match (
            narrative_name,
            single_narrative,
            multi_map,
            &self.narratives,
        ) {
            // Explicit name provided - look in [narrative.name] first, then [narratives.name]
            (Some(name), _, Some(multi), _) if !multi.is_empty() => {
                let entry = multi.get(name).ok_or_else(|| {
                    let available: Vec<_> = multi.keys().cloned().collect();
                    NarrativeErrorKind::NarrativeNotFound {
                        name: name.to_string(),
                        available: available.join(", "),
                    }
                })?;

                self.process_narrative_entry(name, entry)
            }

            // Explicit name, check legacy [narratives.name]
            (Some(name), _, _, narratives) if !narratives.is_empty() => {
                let entry = narratives.get(name).ok_or_else(|| {
                    let available: Vec<_> = narratives.keys().cloned().collect();
                    NarrativeErrorKind::NarrativeNotFound {
                        name: name.to_string(),
                        available: available.join(", "),
                    }
                })?;

                self.process_narrative_entry(name, entry)
            }

            // No name provided, use single [narrative] (backwards compat)
            (None, Some(single), _, _) => {
                let toc = single_toc.ok_or_else(|| {
                    NarrativeErrorKind::MissingRequiredField {
                        field: "toc".to_string(),
                        input_type: "narrative".to_string(),
                    }
                })?;
                Ok((single.clone(), toc.clone(), self.acts.clone()))
            }

            // No name, multiple definitions exist - ambiguous
            (None, _, Some(multi), _) if !multi.is_empty() => {
                let available: Vec<_> = multi.keys().cloned().collect();
                Err(NarrativeErrorKind::AmbiguousNarrative {
                    available: available.join(", "),
                }
                .into())
            }

            // No name, multiple definitions in legacy field - ambiguous
            (None, _, _, narratives) if !narratives.is_empty() => {
                let available: Vec<_> = narratives.keys().cloned().collect();
                Err(NarrativeErrorKind::AmbiguousNarrative {
                    available: available.join(", "),
                }
                .into())
            }

            // No narrative found at all
            _ => Err(NarrativeErrorKind::NoNarrativeFound.into()),
        }
    }

    fn process_narrative_entry(
        &self,
        name: &str,
        entry: &TomlNarrativeEntry,
    ) -> NarrativeResult<(TomlNarrative, TomlToc, HashMap<String, TomlAct>)> {
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
                Ok((meta, toc, acts))
            }
            TomlNarrativeEntry::Reference(_) => Err(NarrativeErrorKind::WrongReferenceType {
                reference: name.to_string(),
                found: "file reference".to_string(),
                expected: "inline definition".to_string(),
            }
            .into()),
        }
    }
}

/// Parse history retention string to HistoryRetention enum.
///
/// Accepts: "full", "summary", "drop"
/// Returns HistoryRetention::Full if None or invalid value.
fn parse_history_retention(value: Option<&String>) -> HistoryRetention {
    match value.map(|s| s.as_str()) {
        Some("full") => HistoryRetention::Full,
        Some("summary") => HistoryRetention::Summary,
        Some("drop") => HistoryRetention::Drop,
        Some(invalid) => {
            warn!(
                value = invalid,
                "Invalid history_retention value, defaulting to 'full'"
            );
            HistoryRetention::Full
        }
        None => HistoryRetention::Full,
    }
}

impl TomlInput {
    /// Convert TOML input to domain Input type.
    #[instrument(skip(self), fields(input_type = ?self.input_type))]
    pub fn to_input(&self) -> NarrativeResult<Input> {
        // Get input type, defaulting to "text" if not specified
        let input_type = self.input_type.as_deref().unwrap_or("text");
        debug!(input_type, "Converting TOML input to domain Input");

        match input_type {
            "text" => {
                // Support both inline content and file reference
                let content = if let Some(content) = &self.content {
                    content.clone()
                } else if let Some(file_ref) = &self.file {
                    // Load content from file
                    let file_path = if Path::new(file_ref).is_absolute()
                        || file_ref.contains('/')
                        || file_ref.contains('\\')
                    {
                        PathBuf::from(file_ref)
                    } else {
                        let context_path = std::env::var("BOTTICELLI_CONTEXT_PATH").ok();
                        match find_file_recursive(file_ref, None, context_path.as_deref()) {
                            Ok(path) => {
                                debug!(original = %file_ref, resolved = %path.display(), "Resolved text file path");
                                path
                            }
                            Err(e) => {
                                warn!(filename = %file_ref, error = %e, "File search failed, trying as-is");
                                PathBuf::from(file_ref)
                            }
                        }
                    };

                    debug!(file = %file_path.display(), "Loading text from file");
                    std::fs::read_to_string(&file_path).map_err(|e| {
                        error!(file = %file_path.display(), error = %e, "Failed to read text file");
                        NarrativeErrorKind::FileRead(format!(
                            "Failed to read text file {}: {}",
                            file_path.display(),
                            e
                        ))
                    })?
                } else {
                    error!("Text input missing both 'content' and 'file' fields");
                    return Err(NarrativeErrorKind::MissingRequiredField {
                        field: "content or file".to_string(),
                        input_type: "text".to_string(),
                    }
                    .into());
                };

                debug!(content_len = content.len(), "Created text input");
                Ok(Input::Text(content))
            }
            "image" => {
                let mime = self.mime.clone();
                let source = self.detect_source()?;
                debug!(?mime, "Created image input");
                Ok(Input::Image { mime, source })
            }
            "audio" => {
                let mime = self.mime.clone();
                let source = self.detect_source()?;
                debug!(?mime, "Created audio input");
                Ok(Input::Audio { mime, source })
            }
            "video" => {
                let mime = self.mime.clone();
                let source = self.detect_source()?;
                debug!(?mime, "Created video input");
                Ok(Input::Video { mime, source })
            }
            "document" => {
                let mime = self.mime.clone();
                let source = self.detect_source()?;
                let filename = self.filename.clone();
                debug!(?mime, ?filename, "Created document input");
                Ok(Input::Document {
                    mime,
                    source,
                    filename,
                })
            }
            "bot_command" => {
                let platform = self.platform.as_ref().ok_or_else(|| {
                    error!("Bot command missing 'platform' field");
                    NarrativeErrorKind::MissingRequiredField {
                        field: "platform".to_string(),
                        input_type: "bot_command".to_string(),
                    }
                })?;
                let command = self.command.as_ref().ok_or_else(|| {
                    error!("Bot command missing 'command' field");
                    NarrativeErrorKind::MissingRequiredField {
                        field: "command".to_string(),
                        input_type: "bot_command".to_string(),
                    }
                })?;
                debug!(%platform, %command, "Created bot command input");
                let args = expand_env_vars(&self.args.clone().unwrap_or_default());
                let history_retention = parse_history_retention(self.history_retention.as_ref());
                Ok(Input::BotCommand {
                    platform: platform.clone(),
                    command: command.clone(),
                    args,
                    required: self.required.unwrap_or(false),
                    cache_duration: self.cache_duration,
                    history_retention,
                })
            }
            "table" => {
                let table_name = self.table_name.as_ref().ok_or_else(|| {
                    error!("Table input missing 'table_name' field");
                    NarrativeErrorKind::MissingRequiredField {
                        field: "table_name".to_string(),
                        input_type: "table".to_string(),
                    }
                })?;

                use botticelli_core::TableFormat;
                let format = match self.format.as_deref() {
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

                debug!(%table_name, ?format, ?self.limit, "Created table input");
                let history_retention = parse_history_retention(self.history_retention.as_ref());
                Ok(Input::Table {
                    table_name: table_name.clone(),
                    columns: self.columns.clone(),
                    where_clause: self.where_clause.clone(),
                    limit: self.limit,
                    offset: self.offset,
                    order_by: self.order_by.clone(),
                    alias: None, // Will be set during resolution
                    format,
                    sample: self.sample,
                    destructive_read: self.pull_and_delete.unwrap_or(true),
                    history_retention,
                })
            }
            unknown => {
                error!(input_type = unknown, "Unknown input type");
                Err(NarrativeErrorKind::InvalidFieldValue {
                    value: unknown.to_string(),
                    field: "type".to_string(),
                    reason: "must be 'text', 'image', 'audio', 'video', 'document', 'bot_command', or 'table'".to_string(),
                }
                .into())
            }
        }
    }

    /// Detect media source from which field is present.
    #[instrument(skip(self))]
    fn detect_source(&self) -> NarrativeResult<MediaSource> {
        if let Some(url) = &self.url {
            debug!(%url, "Using URL source");
            Ok(MediaSource::Url(url.clone()))
        } else if let Some(base64) = &self.base64 {
            debug!(base64_len = base64.len(), "Using base64 source");
            Ok(MediaSource::Base64(base64.clone()))
        } else if let Some(file_ref) = &self.file {
            // Try to find the file recursively if it's just a filename
            let file_path = if Path::new(file_ref).is_absolute()
                || file_ref.contains('/')
                || file_ref.contains('\\')
            {
                // Use as-is if it looks like a full path
                PathBuf::from(file_ref)
            } else {
                // Search recursively for the file
                let context_path = std::env::var("BOTTICELLI_CONTEXT_PATH").ok();
                match find_file_recursive(file_ref, None, context_path.as_deref()) {
                    Ok(path) => {
                        debug!(original = %file_ref, resolved = %path.display(), "Resolved file path");
                        path
                    }
                    Err(e) => {
                        warn!(filename = %file_ref, error = %e, "File search failed, trying as-is");
                        PathBuf::from(file_ref)
                    }
                }
            };

            debug!(file = %file_path.display(), "Reading file source");
            let data = std::fs::read(&file_path).map_err(|e| {
                error!(file = %file_path.display(), error = %e, "Failed to read file");
                NarrativeErrorKind::FileRead(format!(
                    "Failed to read file {}: {}",
                    file_path.display(),
                    e
                ))
            })?;
            debug!(file = %file_path.display(), size = data.len(), "File read successfully");
            Ok(MediaSource::Binary(data))
        } else {
            error!("Media input missing source (url, base64, or file)");
            Err(NarrativeErrorKind::MissingRequiredField {
                field: "url, base64, or file".to_string(),
                input_type: "media".to_string(),
            }
            .into())
        }
    }
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

/// Check if a string is a resource reference (bots.name, tables.name, media.name, narratives.name, narrative:name).
fn is_reference(s: &str) -> bool {
    s.starts_with("bots.")
        || s.starts_with("tables.")
        || s.starts_with("media.")
        || s.starts_with("narratives.")
        || s.starts_with("narrative:")
}

impl TomlNarrativeFile {
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
        let narrative_entry = self.narratives.get(name).ok_or_else(|| {
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

/// Infer MIME type from file extension.
fn infer_mime_type(path: &str) -> Option<String> {
    let extension = std::path::Path::new(path)
        .extension()?
        .to_str()?
        .to_lowercase();

    Some(
        match extension.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "mp3" => "audio/mp3",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "pdf" => "application/pdf",
            "txt" => "text/plain",
            "md" => "text/markdown",
            "json" => "application/json",
            _ => return None,
        }
        .to_string(),
    )
}

/// Infer media type category from extension.
fn infer_media_type_from_extension(path: &str) -> NarrativeResult<&'static str> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| {
            NarrativeErrorKind::InvalidFieldValue {
                value: path.to_string(),
                field: "file".to_string(),
                reason: "cannot determine file extension".to_string(),
            }
        })?
        .to_lowercase();

    Ok(match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "image",
        "mp3" | "wav" | "ogg" => "audio",
        "mp4" | "avi" | "mov" | "webm" => "video",
        "pdf" | "txt" | "md" | "json" => "document",
        _ => {
            return Err(
                NarrativeErrorKind::InvalidFieldValue {
                    value: extension,
                    field: "file extension".to_string(),
                    reason: "unsupported file type".to_string(),
                }
                .into(),
            )
        }
    })
}
