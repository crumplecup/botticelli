//! MCP tools for extraction and parsing operations.
//!
//! Provides tooled access to Extract utility functions for common types
//! used throughout the botticelli workspace.

use anyhow::Result;
use botticelli_narrative::Extract;
use rmcp::tool;
use serde::{Deserialize, Serialize};

/// Parameters for JSON extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractJsonParams {
    /// Response text containing JSON (possibly with markdown or extra text)
    pub response: String,
}

/// Extract JSON from LLM response.
#[tool]
#[tracing::instrument(skip(params), fields(response_len = params.response.len()))]
pub fn extract_json(params: ExtractJsonParams) -> Result<String> {
    Ok(Extract::json(&params.response)?)
}

/// Parameters for TOML extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractTomlParams {
    /// Response text containing TOML (possibly with markdown or extra text)
    pub response: String,
}

/// Extract TOML from LLM response.
#[tool]
#[tracing::instrument(skip(params), fields(response_len = params.response.len()))]
pub fn extract_toml(params: ExtractTomlParams) -> Result<String> {
    Ok(Extract::toml(&params.response)?)
}

// Macro to generate parse_json_<type> tools for all our concrete types
macro_rules! tool_parse_json {
    ($($type_name:ident => $rust_type:ty),* $(,)?) => {
        $(
            paste::paste! {
                #[doc = "Parameters for parsing JSON into " $type_name "."]
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub struct [<Parse $type_name JsonParams>] {
                    /// JSON string to parse
                    pub json_str: String,
                }

                #[doc = "Parse JSON string into " $type_name "."]
                #[tool]
                #[tracing::instrument(skip(params), fields(json_len = params.json_str.len()))]
                pub fn [<parse_ $type_name:snake _json>](
                    params: [<Parse $type_name JsonParams>]
                ) -> Result<$rust_type> {
                    Ok(Extract::parse_json::<$rust_type>(&params.json_str)?)
                }
            }
        )*
    };
}

// Macro to generate parse_toml_<type> tools for all our concrete types
macro_rules! tool_parse_toml {
    ($($type_name:ident => $rust_type:ty),* $(,)?) => {
        $(
            paste::paste! {
                #[doc = "Parameters for parsing TOML into " $type_name "."]
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub struct [<Parse $type_name TomlParams>] {
                    /// TOML string to parse
                    pub toml_str: String,
                }

                #[doc = "Parse TOML string into " $type_name "."]
                #[tool]
                #[tracing::instrument(skip(params), fields(toml_len = params.toml_str.len()))]
                pub fn [<parse_ $type_name:snake _toml>](
                    params: [<Parse $type_name TomlParams>]
                ) -> Result<$rust_type> {
                    Ok(Extract::parse_toml::<$rust_type>(&params.toml_str)?)
                }
            }
        )*
    };
}

// Generate tools for all narrative types (the primitives/grammar)
tool_parse_json! {
    // Core narrative structures
    NarrativeMetadata => botticelli_narrative::NarrativeMetadata,
    NarrativeToc => botticelli_narrative::NarrativeToc,
    ActConfig => botticelli_narrative::ActConfig,
    
    // Execution configuration
    CarouselConfig => botticelli_narrative::CarouselConfig,
    
    // State management
    NarrativeState => botticelli_narrative::NarrativeState,
    StateScope => botticelli_narrative::StateScope,
}

tool_parse_toml! {
    // Full narrative from TOML
    Narrative => botticelli_narrative::Narrative,
    
    // Configuration types that appear in TOML
    CarouselConfig => botticelli_narrative::CarouselConfig,
    ActConfig => botticelli_narrative::ActConfig,
}

// Generate tools for common std types we use
tool_parse_json! {
    StringVec => Vec<String>,
    StringMap => std::collections::HashMap<String, String>,
    JsonValue => serde_json::Value,
}
