//! Input type and conversion logic.

use rmcp::tool;
use super::utils::*;
use botticelli_core::{Input, MediaSource, TableFormat};
use botticelli_error::{IoError, NarrativeErrorKind, NarrativeResult};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, error, instrument, warn};

/// TOML representation of an input.
///
/// The `type` field determines which other fields are required.
/// Source is detected from which of url/base64/file is present.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters, elicitation::Elicit)]
pub struct TomlInput {
    /// Input type: "text", "image", "audio", "video", "document", "bot_command", "table"
    #[serde(rename = "type")]
    pub(super) input_type: Option<String>,

    /// Reference to a resource: "bots.name", "tables.name", "media.name"
    #[serde(rename = "ref")]
    pub(super) reference: Option<String>,

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

impl TomlInput {
    /// Convert TOML input to domain Input type.
    #[instrument(skip(self), fields(input_type = ?self.input_type))]
    #[tool]
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
                        NarrativeErrorKind::Io(IoError::from(e))
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
                NarrativeErrorKind::Io(IoError::from(e))
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
