//! Narrative resource for TOML narrative files.

use super::{McpResource, ResourceInfo};
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, instrument};

/// Resource for accessing narrative TOML files.
///
/// URI format: `narrative://{name}`
/// Example: `narrative://curate_content`
pub struct NarrativeResource {
    narratives_dir: PathBuf,
}

impl NarrativeResource {
    /// Creates a new narrative resource with default directory.
    pub fn new() -> Self {
        Self {
            narratives_dir: PathBuf::from("narratives"),
        }
    }

    /// Creates a new narrative resource with custom directory.
    pub fn with_directory(narratives_dir: impl Into<PathBuf>) -> Self {
        Self {
            narratives_dir: narratives_dir.into(),
        }
    }

    /// Parses a narrative URI into a name.
    fn parse_uri(&self, uri: &str) -> McpResult<String> {
        let name = uri.strip_prefix("narrative://").ok_or_else(|| {
            McpError::resource_not_found(
                "Invalid narrative URI: missing narrative:// scheme".to_string(),
            )
        })?;

        if name.is_empty() {
            return Err(McpError::invalid_input(
                "Invalid narrative URI: empty name".to_string(),
            ));
        }

        Ok(name.to_string())
    }

    /// Gets the path to a narrative file.
    fn narrative_path(&self, name: &str) -> PathBuf {
        self.narratives_dir.join(format!("{}.toml", name))
    }

    /// Reads a narrative TOML file.
    #[instrument(skip(self))]
    fn read_narrative(&self, name: &str) -> McpResult<String> {
        let path = self.narrative_path(name);
        debug!(path = %path.display(), "Reading narrative file");

        fs::read_to_string(&path).map_err(|e| {
            McpError::resource_not_found(format!(
                "Failed to read narrative '{}' at {}: {}",
                name,
                path.display(),
                e
            ))
        })
    }

    /// Lists available narrative files.
    #[instrument(skip(self))]
    fn list_narratives(&self) -> McpResult<Vec<String>> {
        if !self.narratives_dir.exists() {
            return Ok(vec![]);
        }

        let entries = fs::read_dir(&self.narratives_dir).map_err(|e| {
            McpError::execution_failed(format!(
                "Failed to read narratives directory {}: {}",
                self.narratives_dir.display(),
                e
            ))
        })?;

        let mut narratives = Vec::new();
        for entry in entries {
            let entry = entry
                .map_err(|e| McpError::execution_failed(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    narratives.push(name.to_string());
                }
            }
        }

        narratives.sort();
        Ok(narratives)
    }
}

impl Default for NarrativeResource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpResource for NarrativeResource {
    fn uri_pattern(&self) -> &'static str {
        "narrative://"
    }

    fn description(&self) -> &'static str {
        "Access narrative TOML configuration files"
    }

    #[instrument(skip(self), fields(uri))]
    async fn read(&self, uri: &str) -> McpResult<String> {
        let name = self.parse_uri(uri)?;
        debug!(name, "Reading narrative");
        self.read_narrative(&name)
    }

    #[instrument(skip(self))]
    async fn list(&self) -> McpResult<Vec<ResourceInfo>> {
        let narratives = self.list_narratives()?;

        let resources = narratives
            .into_iter()
            .map(|name| ResourceInfo {
                uri: format!("narrative://{}", name),
                name: name.clone(),
                description: format!("Narrative configuration: {}", name),
                mime_type: Some("application/toml".to_string()),
            })
            .collect();

        Ok(resources)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uri() {
        let resource = NarrativeResource::new();

        let name = resource
            .parse_uri("narrative://curate_content")
            .expect("Valid URI");
        assert_eq!(name, "curate_content");
    }

    #[test]
    fn test_parse_uri_invalid() {
        let resource = NarrativeResource::new();

        assert!(resource.parse_uri("invalid://uri").is_err());
        assert!(resource.parse_uri("narrative://").is_err());
    }

    #[test]
    fn test_narrative_path() {
        let resource = NarrativeResource::new();
        let path = resource.narrative_path("test");
        assert_eq!(path, PathBuf::from("narratives/test.toml"));
    }
}
