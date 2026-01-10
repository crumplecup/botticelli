//! Filesystem-based narrative storage implementation.
//!
//! Provides file system access to narrative TOML files for MCP tool integration.

use async_trait::async_trait;
use botticelli_error::NarrativeError;
use botticelli_interface::NarrativeStorageOperations;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// Filesystem storage provider for narratives.
#[derive(Debug, Clone, derive_new::new)]
pub struct FilesystemNarrativeStorage {
    narrative_dir: PathBuf,
}

#[async_trait]
impl NarrativeStorageOperations for FilesystemNarrativeStorage {
    type Error = NarrativeError;

    #[tracing::instrument(skip(self), fields(pattern))]
    async fn list_narratives(&self, pattern: Option<&str>) -> Result<Vec<String>, Self::Error> {
        let mut entries = fs::read_dir(&self.narrative_dir).await?;

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
    async fn load_narrative(&self, filename: &str) -> Result<Value, Self::Error> {
        let path = self.narrative_dir.join(filename);
        let toml_content = fs::read_to_string(&path).await?;
        self.parse_narrative(&toml_content, None).await
    }

    #[tracing::instrument(skip(self, toml_content))]
    async fn validate_narrative(&self, toml_content: &str) -> Result<Value, Self::Error> {
        // Parse as Narrative to validate structure
        let narrative: crate::Narrative = toml_content.parse()?;

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
    ) -> Result<Value, Self::Error> {
        // Parse using FromStr implementation
        let narrative: crate::Narrative = if let Some(name) = name_override {
            crate::Narrative::from_toml_str(toml_content, Some(name))?
        } else {
            toml_content.parse()?
        };

        // Convert to JSON for MCP response
        Ok(serde_json::to_value(&narrative)?)
    }
}
