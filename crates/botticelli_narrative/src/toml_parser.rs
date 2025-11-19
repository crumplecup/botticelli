//! TOML deserialization structures for narrative configuration.
//!
//! This module provides intermediate structures for deserializing TOML
//! into our domain types (ActConfig, Input, etc.).

use crate::ActConfig;
use botticelli_core::{Input, MediaSource};
use serde::Deserialize;
use std::collections::HashMap;

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

/// Intermediate structure for deserializing acts.
///
/// Acts can be either:
/// - Simple strings: `act_name = "prompt text"`
/// - Structured tables: `[acts.act_name]` with optional `[[acts.act_name.input]]`
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlAct {
    /// Simple text act: `act_name = "prompt"`
    Simple(String),
    /// Structured act with configuration
    Structured(TomlActConfig),
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
    /// Input type: "text", "image", "audio", "video", "document"
    #[serde(rename = "type")]
    pub input_type: String,

    // Text input field
    pub content: Option<String>,

    // Media input fields
    pub mime: Option<String>,
    pub url: Option<String>,
    pub base64: Option<String>,
    pub file: Option<String>,

    // Document-specific field
    pub filename: Option<String>,
}

/// Root TOML structure.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlNarrativeFile {
    pub narrative: TomlNarrative,
    pub toc: TomlToc,
    pub acts: HashMap<String, TomlAct>,
}

impl TomlInput {
    /// Convert TOML input to domain Input type.
    pub fn to_input(&self) -> Result<Input, String> {
        match self.input_type.as_str() {
            "text" => {
                let content = self
                    .content
                    .as_ref()
                    .ok_or("Text input missing 'content' field")?;
                Ok(Input::Text(content.clone()))
            }
            "image" => {
                let mime = self.mime.clone();
                let source = self.detect_source()?;
                Ok(Input::Image { mime, source })
            }
            "audio" => {
                let mime = self.mime.clone();
                let source = self.detect_source()?;
                Ok(Input::Audio { mime, source })
            }
            "video" => {
                let mime = self.mime.clone();
                let source = self.detect_source()?;
                Ok(Input::Video { mime, source })
            }
            "document" => {
                let mime = self.mime.clone();
                let source = self.detect_source()?;
                let filename = self.filename.clone();
                Ok(Input::Document {
                    mime,
                    source,
                    filename,
                })
            }
            unknown => Err(format!("Unknown input type: {}", unknown)),
        }
    }

    /// Detect media source from which field is present.
    fn detect_source(&self) -> Result<MediaSource, String> {
        if let Some(url) = &self.url {
            Ok(MediaSource::Url(url.clone()))
        } else if let Some(base64) = &self.base64 {
            Ok(MediaSource::Base64(base64.clone()))
        } else if let Some(file) = &self.file {
            Ok(MediaSource::Binary(std::fs::read(file).map_err(|e| {
                format!("Failed to read file {}: {}", file, e)
            })?))
        } else {
            Err("Media input missing source (url, base64, or file)".to_string())
        }
    }
}

impl TomlActConfig {
    /// Convert TOML act config to domain ActConfig.
    pub fn to_act_config(&self) -> Result<ActConfig, String> {
        let inputs: Result<Vec<Input>, String> =
            self.input.iter().map(|ti| ti.to_input()).collect();

        Ok(ActConfig {
            inputs: inputs?,
            model: self.model.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        })
    }
}

impl TomlAct {
    /// Convert TOML act to domain ActConfig.
    pub fn to_act_config(&self) -> Result<ActConfig, String> {
        match self {
            TomlAct::Simple(text) => {
                // Validate that the text is not empty or just whitespace
                if text.trim().is_empty() {
                    return Err("Act prompt cannot be empty or whitespace only".to_string());
                }
                Ok(ActConfig::from_text(text.clone()))
            }
            TomlAct::Structured(config) => config.to_act_config(),
        }
    }
}
