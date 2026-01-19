//! MCP tools for extraction and parsing operations.
//!
//! Provides tooled access to Extract utility functions for common types
//! used throughout the botticelli workspace.

use anyhow::Result;
use botticelli_narrative::Extract;
use crate::rmcp_server::BotticelliServer;
use rmcp::tool;
use serde::{Deserialize, Serialize};

/// Parameters for JSON extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractJsonParams {
    /// Response text containing JSON (possibly with markdown or extra text)
    pub response: String,
}

/// Parameters for TOML extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractTomlParams {
    /// Response text containing TOML (possibly with markdown or extra text)
    pub response: String,
}

// Macro to generate parse_json_<type> parameter structs
macro_rules! json_params {
    ($($type_name:ident),* $(,)?) => {
        $(
            paste::paste! {
                #[doc = "Parameters for parsing JSON into " $type_name "."]
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub struct [<Parse $type_name JsonParams>] {
                    /// JSON string to parse
                    pub json_str: String,
                }
            }
        )*
    };
}

// Macro to generate parse_toml_<type> parameter structs
macro_rules! toml_params {
    ($($type_name:ident),* $(,)?) => {
        $(
            paste::paste! {
                #[doc = "Parameters for parsing TOML into " $type_name "."]
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub struct [<Parse $type_name TomlParams>] {
                    /// TOML string to parse
                    pub toml_str: String,
                }
            }
        )*
    };
}

// Generate parameter structs for all types
json_params! {
    NarrativeMetadata,
    NarrativeToc,
    ActConfig,
    CarouselConfig,
    NarrativeState,
    StateScope,
    StringVec,
    StringMap,
    JsonValue,
}

toml_params! {
    Narrative,
    CarouselConfig,
    ActConfig,
}

// Macro to generate parse_json_<type> methods (invoked inside impl block)
macro_rules! impl_parse_json {
    ($($type_name:ident => $rust_type:ty),* $(,)?) => {
        $(
            paste::paste! {
                #[doc = "Parse JSON string into " $type_name "."]
                #[tool]
                #[tracing::instrument(skip(self, params), fields(json_len = params.json_str.len()))]
                pub fn [<parse_ $type_name:snake _json>](
                    &self,
                    params: [<Parse $type_name JsonParams>]
                ) -> Result<$rust_type> {
                    Ok(Extract::parse_json::<$rust_type>(&params.json_str)?)
                }
            }
        )*
    };
}

// Macro to generate parse_toml_<type> methods (invoked inside impl block)
macro_rules! impl_parse_toml {
    ($($type_name:ident => $rust_type:ty),* $(,)?) => {
        $(
            paste::paste! {
                #[doc = "Parse TOML string into " $type_name "."]
                #[tool]
                #[tracing::instrument(skip(self, params), fields(toml_len = params.toml_str.len()))]
                pub fn [<parse_ $type_name:snake _toml>](
                    &self,
                    params: [<Parse $type_name TomlParams>]
                ) -> Result<$rust_type> {
                    Ok(Extract::parse_toml::<$rust_type>(&params.toml_str)?)
                }
            }
        )*
    };
}

impl BotticelliServer {
    /// Extract JSON from LLM response.
    ///
    /// Removes markdown code fences and extracts clean JSON.
    #[tool]
    #[tracing::instrument(skip(self, params), fields(response_len = params.response.len()))]
    pub fn extract_json(&self, params: ExtractJsonParams) -> Result<String> {
        Ok(Extract::json(&params.response)?)
    }

    /// Extract TOML from LLM response.
    ///
    /// Removes markdown code fences and extracts clean TOML.
    #[tool]
    #[tracing::instrument(skip(self, params), fields(response_len = params.response.len()))]
    pub fn extract_toml(&self, params: ExtractTomlParams) -> Result<String> {
        Ok(Extract::toml(&params.response)?)
    }

    // Generate methods for all narrative types
    impl_parse_json! {
        NarrativeMetadata => botticelli_narrative::NarrativeMetadata,
        NarrativeToc => botticelli_narrative::NarrativeToc,
        ActConfig => botticelli_narrative::ActConfig,
        CarouselConfig => botticelli_narrative::CarouselConfig,
        NarrativeState => botticelli_narrative::NarrativeState,
        StateScope => botticelli_narrative::StateScope,
    }

    impl_parse_toml! {
        Narrative => botticelli_narrative::Narrative,
        CarouselConfig => botticelli_narrative::CarouselConfig,
        ActConfig => botticelli_narrative::ActConfig,
    }

    // Generate methods for common std types
    impl_parse_json! {
        StringVec => Vec<String>,
        StringMap => std::collections::HashMap<String, String>,
        JsonValue => serde_json::Value,
    }
}
