//! Filesystem-based narrative storage implementation.
//!
//! Provides file system access to narrative TOML files for MCP tool integration.

use async_trait::async_trait;
use botticelli_error::{BotticelliError, BotticelliResult, NarrativeError};
use botticelli_interface::NarrativeStorageOperations;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// Filesystem storage provider for narratives.
#[derive(Debug, Clone)]
pub struct FilesystemNarrativeStorage {
    narrative_dir: PathBuf,
}

impl FilesystemNarrativeStorage {
    /// Create new filesystem storage with directory path.
    pub fn new(narrative_dir: PathBuf) -> Self {
        Self { narrative_dir }
    }
}

#[async_trait]
impl NarrativeStorageOperations for FilesystemNarrativeStorage {
    #[tracing::instrument(skip(self), fields(pattern))]
    async fn list_narratives(&self, pattern: Option<&str>) -> BotticelliResult<Vec<String>> {
        let mut entries = fs::read_dir(&self.narrative_dir).await.map_err(|e| {
            BotticelliError::from(botticelli_error::BackendError::new(format!(
                "Failed to read narrative directory: {}",
                e
            )))
        })?;

        let mut narratives = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml")
                && let Some(filename) = path.file_name().and_then(|s| s.to_str())
            {
                // Apply pattern filter if provided
                if let Some(pat) = pattern {
                    if filename.contains(pat) {
                        narratives.push(filename.to_string());
                    }
                } else {
                    narratives.push(filename.to_string());
                }
            }
        }

        narratives.sort();
        Ok(narratives)
    }

    #[tracing::instrument(skip(self), fields(filename))]
    async fn load_narrative(&self, filename: &str) -> BotticelliResult<Value> {
        let path = self.narrative_dir.join(filename);

        let toml_content = fs::read_to_string(&path).await.map_err(|e| {
            BotticelliError::from(botticelli_error::BackendError::new(format!(
                "Failed to read narrative file: {}",
                e
            )))
        })?;

        self.parse_narrative(&toml_content, None).await
    }

    #[tracing::instrument(skip(self, toml_content))]
    async fn validate_narrative(&self, toml_content: &str) -> BotticelliResult<Value> {
        // Parse as Narrative to validate structure
        let narrative: crate::Narrative = toml_content.parse().map_err(|e: NarrativeError| {
            BotticelliError::from(botticelli_error::BackendError::new(format!(
                "Invalid narrative TOML: {}",
                e
            )))
        })?;

        // Return validation result
        Ok(serde_json::json!({
            "valid": true,
            "name": narrative.metadata().name(),
            "description": narrative.metadata().description(),
            "act_count": narrative.acts().len(),
        }))
    }

    #[tracing::instrument(skip(self, toml_content))]
    async fn parse_narrative(
        &self,
        toml_content: &str,
        name_override: Option<&str>,
    ) -> BotticelliResult<Value> {
        // Parse using FromStr implementation
        let narrative: crate::Narrative = if let Some(name) = name_override {
            crate::Narrative::from_toml_str(toml_content, Some(name)).map_err(|e| {
                BotticelliError::from(botticelli_error::BackendError::new(format!(
                    "Failed to parse narrative TOML: {}",
                    e
                )))
            })?
        } else {
            toml_content.parse().map_err(|e: NarrativeError| {
                BotticelliError::from(botticelli_error::BackendError::new(format!(
                    "Failed to parse narrative TOML: {}",
                    e
                )))
            })?
        };

        // Convert to JSON for MCP response
        serde_json::to_value(&narrative).map_err(|e| {
            BotticelliError::from(botticelli_error::BackendError::new(format!(
                "Failed to serialize narrative: {}",
                e
            )))
        })
    }
}
