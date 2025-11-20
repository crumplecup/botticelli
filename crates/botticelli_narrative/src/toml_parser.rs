//! TOML deserialization structures for narrative configuration.
//!
//! This module provides intermediate structures for deserializing TOML
//! into our domain types (ActConfig, Input, etc.).

use crate::ActConfig;
use botticelli_core::{Input, MediaSource};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, error, instrument};

/// Intermediate structure for deserializing the [narrative] section.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlNarrative {
    pub name: String,
    pub description: String,
    /// Optional template table to use as schema source for content generation
    pub template: Option<String>,
    /// Optional flag to skip content generation (both template and inference modes)
    #[serde(default)]
    pub skip_content_generation: bool,
}

/// Intermediate structure for deserializing the [toc] section.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlToc {
    pub order: Vec<String>,
}

/// Bot command definition from [bots.name] section.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlBotDefinition {
    pub platform: String,
    pub command: String,
    /// All other fields are flattened into args
    #[serde(flatten)]
    pub args: HashMap<String, serde_json::Value>,
}

/// Table query definition from [tables.name] section.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlTableDefinition {
    pub table_name: String,
    pub columns: Option<Vec<String>>,
    #[serde(rename = "where")]
    pub where_clause: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<String>,
    pub format: Option<String>,
    pub sample: Option<u32>,
}

/// Media source definition from [media.name] section.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlMediaDefinition {
    pub url: Option<String>,
    pub file: Option<String>,
    pub base64: Option<String>,
    pub mime: Option<String>,
    pub filename: Option<String>,
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
    Structured(TomlInput),
}

/// Structured act configuration from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlActConfig {
    /// Array of inputs via `[[acts.act_name.input]]` syntax
    #[serde(default)]
    pub input: Vec<TomlInput>,

    /// Optional model override
    pub model: Option<String>,

    /// Optional temperature override
    pub temperature: Option<f32>,

    /// Optional max_tokens override
    pub max_tokens: Option<u32>,
}

/// TOML representation of an input.
///
/// The `type` field determines which other fields are required.
/// Source is detected from which of url/base64/file is present.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlInput {
    /// Input type: "text", "image", "audio", "video", "document", "bot_command", "table"
    #[serde(rename = "type")]
    pub input_type: Option<String>,

    /// Reference to a resource: "bots.name", "tables.name", "media.name"
    #[serde(rename = "ref")]
    pub reference: Option<String>,

    // Text input field
    pub content: Option<String>,

    // Media input fields
    pub mime: Option<String>,
    pub url: Option<String>,
    pub base64: Option<String>,
    pub file: Option<String>,

    // Document-specific field
    pub filename: Option<String>,

    // Bot command fields
    pub platform: Option<String>,
    pub command: Option<String>,
    pub args: Option<HashMap<String, serde_json::Value>>,
    pub required: Option<bool>,
    pub cache_duration: Option<u64>,

    // Table reference fields
    pub table_name: Option<String>,
    pub columns: Option<Vec<String>>,
    #[serde(rename = "where")]
    pub where_clause: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<String>,
    pub format: Option<String>,
    pub sample: Option<u32>,
}

/// Root TOML structure.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlNarrativeFile {
    pub narrative: TomlNarrative,
    pub toc: TomlToc,
    pub acts: HashMap<String, TomlAct>,
    /// Optional bot command definitions
    #[serde(default)]
    pub bots: HashMap<String, TomlBotDefinition>,
    /// Optional table query definitions
    #[serde(default)]
    pub tables: HashMap<String, TomlTableDefinition>,
    /// Optional media source definitions
    #[serde(default)]
    pub media: HashMap<String, TomlMediaDefinition>,
}

impl TomlInput {
    /// Convert TOML input to domain Input type.
    #[instrument(skip(self), fields(input_type = ?self.input_type))]
    pub fn to_input(&self) -> Result<Input, String> {
        // Get input type, defaulting to "text" if not specified
        let input_type = self.input_type.as_deref().unwrap_or("text");
        debug!(input_type, "Converting TOML input to domain Input");
        
        match input_type {
            "text" => {
                let content = self
                    .content
                    .as_ref()
                    .ok_or_else(|| {
                        error!("Text input missing 'content' field");
                        "Text input missing 'content' field".to_string()
                    })?;
                debug!(content_len = content.len(), "Created text input");
                Ok(Input::Text(content.clone()))
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
                let platform = self
                    .platform
                    .as_ref()
                    .ok_or_else(|| {
                        error!("Bot command missing 'platform' field");
                        "Bot command missing 'platform' field".to_string()
                    })?;
                let command = self
                    .command
                    .as_ref()
                    .ok_or_else(|| {
                        error!("Bot command missing 'command' field");
                        "Bot command missing 'command' field".to_string()
                    })?;
                debug!(%platform, %command, "Created bot command input");
                Ok(Input::BotCommand {
                    platform: platform.clone(),
                    command: command.clone(),
                    args: self.args.clone().unwrap_or_default(),
                    required: self.required.unwrap_or(false),
                    cache_duration: self.cache_duration,
                })
            }
            "table" => {
                let table_name = self
                    .table_name
                    .as_ref()
                    .ok_or_else(|| {
                        error!("Table input missing 'table_name' field");
                        "Table input missing 'table_name' field".to_string()
                    })?;
                
                use botticelli_core::TableFormat;
                let format = match self.format.as_deref() {
                    Some("json") | None => TableFormat::Json,
                    Some("markdown") => TableFormat::Markdown,
                    Some("csv") => TableFormat::Csv,
                    Some(f) => {
                        error!(format = f, "Unknown table format");
                        return Err(format!("Unknown table format: {}", f));
                    }
                };
                
                debug!(%table_name, ?format, ?self.limit, "Created table input");
                Ok(Input::Table {
                    table_name: table_name.clone(),
                    columns: self.columns.clone(),
                    where_clause: self.where_clause.clone(),
                    limit: self.limit,
                    offset: self.offset,
                    order_by: self.order_by.clone(),
                    alias: None,  // Will be set during resolution
                    format,
                    sample: self.sample,
                })
            }
            unknown => {
                error!(input_type = unknown, "Unknown input type");
                Err(format!("Unknown input type: {}", unknown))
            }
        }
    }

    /// Detect media source from which field is present.
    #[instrument(skip(self))]
    fn detect_source(&self) -> Result<MediaSource, String> {
        if let Some(url) = &self.url {
            debug!(%url, "Using URL source");
            Ok(MediaSource::Url(url.clone()))
        } else if let Some(base64) = &self.base64 {
            debug!(base64_len = base64.len(), "Using base64 source");
            Ok(MediaSource::Base64(base64.clone()))
        } else if let Some(file) = &self.file {
            debug!(%file, "Reading file source");
            let data = std::fs::read(file).map_err(|e| {
                error!(%file, error = %e, "Failed to read file");
                format!("Failed to read file {}: {}", file, e)
            })?;
            debug!(%file, size = data.len(), "File read successfully");
            Ok(MediaSource::Binary(data))
        } else {
            error!("Media input missing source (url, base64, or file)");
            Err("Media input missing source (url, base64, or file)".to_string())
        }
    }
}

impl TomlActConfig {
    /// Convert TOML act config to domain ActConfig.
    #[allow(dead_code)]
    #[instrument(skip(self), fields(input_count = self.input.len()))]
    pub fn to_act_config(&self) -> Result<ActConfig, String> {
        debug!("Converting TOML act config to domain ActConfig");
        let inputs: Result<Vec<Input>, String> =
            self.input.iter().map(|ti| ti.to_input()).collect();

        let inputs = inputs?;
        debug!(inputs_converted = inputs.len(), "Successfully converted inputs");
        Ok(ActConfig {
            inputs,
            model: self.model.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        })
    }
}

impl TomlAct {
    /// Convert TOML act to domain ActConfig.
    /// 
    /// Requires the parent TomlNarrativeFile for resolving references.
    #[instrument(skip(self, narrative_file))]
    pub fn to_act_config(&self, narrative_file: &TomlNarrativeFile) -> Result<ActConfig, String> {
        debug!("Converting TOML act to domain ActConfig");
        match self {
            TomlAct::Simple(text) => {
                // Check if it's a resource reference
                if is_reference(text) {
                    debug!(reference = %text, "Resolving simple reference");
                    let input = narrative_file.resolve_reference(text)?;
                    Ok(ActConfig {
                        inputs: vec![input],
                        model: None,
                        temperature: None,
                        max_tokens: None,
                    })
                } else {
                    // Validate that the text is not empty or just whitespace
                    if text.trim().is_empty() {
                        error!("Act prompt cannot be empty or whitespace only");
                        return Err("Act prompt cannot be empty or whitespace only".to_string());
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
                debug!(input_count = inputs.len(), "Array act converted successfully");
                Ok(ActConfig {
                    inputs,
                    model: None,
                    temperature: None,
                    max_tokens: None,
                })
            }
            TomlAct::Structured(config) => {
                debug!(input_count = config.input.len(), "Processing structured act");
                // Handle references in structured inputs
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
                debug!(input_count = inputs.len(), "Structured act converted successfully");
                Ok(ActConfig {
                    inputs,
                    model: config.model.clone(),
                    temperature: config.temperature,
                    max_tokens: config.max_tokens,
                })
            }
        }
    }
}

/// Check if a string is a resource reference (bots.name, tables.name, media.name).
fn is_reference(s: &str) -> bool {
    s.starts_with("bots.") || s.starts_with("tables.") || s.starts_with("media.")
}

impl TomlNarrativeFile {
    /// Resolve a resource reference to an Input.
    #[instrument(skip(self), fields(reference))]
    pub fn resolve_reference(&self, reference: &str) -> Result<Input, String> {
        debug!(%reference, "Resolving resource reference");
        let parts: Vec<&str> = reference.split('.').collect();
        if parts.len() != 2 {
            error!(%reference, "Invalid reference format (expected 'category.name')");
            return Err(format!("Invalid reference format: {}", reference));
        }
        
        let (category, name) = (parts[0], parts[1]);
        debug!(category, name, "Parsed reference");
        
        match category {
            "bots" => self.resolve_bot_reference(name),
            "tables" => self.resolve_table_reference(name),
            "media" => self.resolve_media_reference(name),
            _ => {
                error!(category, "Unknown reference category");
                Err(format!("Unknown reference category: {}", category))
            }
        }
    }
    
    #[instrument(skip(self), fields(name))]
    fn resolve_bot_reference(&self, name: &str) -> Result<Input, String> {
        debug!(%name, "Resolving bot reference");
        let bot_def = self.bots.get(name)
            .ok_or_else(|| {
                error!(%name, "Bot not found");
                format!("Bot not found: {}", name)
            })?;
        
        debug!(platform = %bot_def.platform, command = %bot_def.command, "Bot reference resolved");
        Ok(Input::BotCommand {
            platform: bot_def.platform.clone(),
            command: bot_def.command.clone(),
            args: bot_def.args.clone(),
            required: false,
            cache_duration: None,
        })
    }
    
    #[instrument(skip(self), fields(name))]
    fn resolve_table_reference(&self, name: &str) -> Result<Input, String> {
        debug!(%name, "Resolving table reference");
        let table_def = self.tables.get(name)
            .ok_or_else(|| {
                error!(%name, "Table not found");
                format!("Table not found: {}", name)
            })?;
        
        use botticelli_core::TableFormat;
        let format = match table_def.format.as_deref() {
            Some("json") | None => TableFormat::Json,
            Some("markdown") => TableFormat::Markdown,
            Some("csv") => TableFormat::Csv,
            Some(f) => {
                error!(format = f, "Unknown table format");
                return Err(format!("Unknown table format: {}", f));
            }
        };
        
        debug!(
            table_name = %table_def.table_name, 
            ?format, 
            ?table_def.limit, 
            "Table reference resolved"
        );
        Ok(Input::Table {
            table_name: table_def.table_name.clone(),
            columns: table_def.columns.clone(),
            where_clause: table_def.where_clause.clone(),
            limit: table_def.limit,
            offset: table_def.offset,
            order_by: table_def.order_by.clone(),
            alias: Some(name.to_string()),
            format,
            sample: table_def.sample,
        })
    }
    
    #[instrument(skip(self), fields(name))]
    fn resolve_media_reference(&self, name: &str) -> Result<Input, String> {
        debug!(%name, "Resolving media reference");
        let media_def = self.media.get(name)
            .ok_or_else(|| {
                error!(%name, "Media not found");
                format!("Media not found: {}", name)
            })?;
        
        // Detect media source
        let source = if let Some(url) = &media_def.url {
            debug!(%url, "Using URL source");
            MediaSource::Url(url.clone())
        } else if let Some(file) = &media_def.file {
            debug!(%file, "Reading file source");
            let data = std::fs::read(file).map_err(|e| {
                error!(%file, error = %e, "Failed to read file");
                format!("Failed to read file {}: {}", file, e)
            })?;
            debug!(%file, size = data.len(), "File read successfully");
            MediaSource::Binary(data)
        } else if let Some(base64) = &media_def.base64 {
            debug!(base64_len = base64.len(), "Using base64 source");
            MediaSource::Base64(base64.clone())
        } else {
            error!(%name, "Media definition missing source (url, file, or base64)");
            return Err(format!("Media definition '{}' missing source (url, file, or base64)", name));
        };
        
        // Infer MIME type if not provided
        let mime = media_def.mime.clone().or_else(|| {
            media_def.file.as_ref()
                .or(media_def.url.as_ref())
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
                    return Err(format!("Cannot determine media type from MIME: {}", mime_str));
                }
            }
        } else {
            // Infer from file extension
            let path = media_def.file.as_ref()
                .or(media_def.url.as_ref())
                .ok_or_else(|| {
                    error!(%name, "Cannot infer media type without file path or MIME");
                    format!("Cannot infer media type for '{}' without file path or MIME", name)
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
                filename: media_def.filename.clone(),
            }),
            _ => {
                error!(media_type, "Unknown media type");
                Err(format!("Unknown media type: {}", media_type))
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
    
    Some(match extension.as_str() {
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
    }.to_string())
}

/// Infer media type category from extension.
fn infer_media_type_from_extension(path: &str) -> Result<&'static str, String> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| format!("Cannot determine file extension from: {}", path))?
        .to_lowercase();
    
    Ok(match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "image",
        "mp3" | "wav" | "ogg" => "audio",
        "mp4" | "avi" | "mov" | "webm" => "video",
        "pdf" | "txt" | "md" | "json" => "document",
        _ => return Err(format!("Unknown file extension: {}", extension)),
    })
}
