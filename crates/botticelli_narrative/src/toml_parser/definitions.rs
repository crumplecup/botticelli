//! Definition types for bots, tables, media, and narrative references.

use serde::Deserialize;
use std::collections::HashMap;

/// Bot command definition from [bots.name] section.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters, elicitation::Elicit)]
pub struct TomlBotDefinition {
    platform: String,
    command: String,
    /// All other fields are flattened into args
    #[serde(flatten)]
    args: HashMap<String, serde_json::Value>,
}

/// Table query definition from [tables.name] section.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters, elicitation::Elicit)]
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
#[derive(Debug, Clone, Deserialize, derive_getters::Getters, elicitation::Elicit)]
pub struct TomlMediaDefinition {
    url: Option<String>,
    file: Option<String>,
    base64: Option<String>,
    mime: Option<String>,
    filename: Option<String>,
}

/// Nested narrative reference from [narratives.name] section.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters, elicitation::Elicit)]
pub struct TomlNarrativeReference {
    narrative: String,
}
