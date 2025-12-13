//! Tool for saving narratives to files.

use crate::tools::McpTool;
use botticelli_error::{McpError, McpResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use tracing::{debug, instrument};

/// Tool for saving narrative TOML to files.
///
/// Takes a narrative TOML string and saves it to the specified path.
/// Supports overwrite protection and path validation.
pub struct SaveNarrativeTool;

#[async_trait]
impl McpTool for SaveNarrativeTool {
    fn name(&self) -> &str {
        "save_narrative"
    }

    fn description(&self) -> &str {
        "Save a narrative TOML to a file. \
         Validates the path and provides overwrite protection. \
         Returns confirmation of the saved file location."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_toml": {
                    "type": "string",
                    "description": "Narrative TOML content to save"
                },
                "file_path": {
                    "type": "string",
                    "description": "Path where to save the file (should end in .toml)"
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "Allow overwriting existing file",
                    "default": false
                }
            },
            "required": ["narrative_toml", "file_path"]
        })
    }

    #[instrument(skip(self, input))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        debug!("Saving narrative to file");

        // Extract inputs
        let narrative_toml = input
            .get("narrative_toml")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'narrative_toml'".to_string()))?;

        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'file_path'".to_string()))?;

        let overwrite = input
            .get("overwrite")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Validate path
        let path = Path::new(file_path);

        // Check extension
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            return Err(McpError::invalid_input(
                "File path must end with .toml extension".to_string(),
            ));
        }

        // Check if file exists
        if path.exists() && !overwrite {
            return Err(McpError::execution_failed(format!(
                "File '{}' already exists. Set overwrite=true to replace it.",
                file_path
            )));
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    McpError::execution_failed(format!("Failed to create directories: {}", e))
                })?;
                debug!(path = ?parent, "Created parent directories");
            }
        }

        // Write file
        tokio::fs::write(path, narrative_toml).await.map_err(|e| {
            McpError::execution_failed(format!("Failed to write file: {}", e))
        })?;

        debug!(path = file_path, "Narrative saved to file");

        // Get absolute path for response
        let absolute_path = std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.to_string());

        Ok(json!({
            "status": "saved",
            "file_path": absolute_path,
            "size_bytes": narrative_toml.len(),
            "overwritten": path.metadata().is_ok()
        }))
    }
}
