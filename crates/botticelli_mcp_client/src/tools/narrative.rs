//! Narrative generation and execution tools.
//!
//! Exposes Botticelli's narrative system as MCP tools for LLM orchestration.

use crate::{McpClientError, McpClientErrorKind, McpClientResult, ToolHandler};
use async_trait::async_trait;
use botticelli_narrative::Narrative;
use pmcp::{Content, ToolInfo};
use serde_json::{Value, json};

/// Tool for creating narratives from TOML content.
///
/// Parses TOML and validates narrative structure.
pub struct CreateNarrativeTool<S: botticelli_interface::NarrativeStorageOperations> {
    storage: S,
}

impl<S: botticelli_interface::NarrativeStorageOperations> CreateNarrativeTool<S> {
    /// Create new create narrative tool.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl<S: botticelli_interface::NarrativeStorageOperations> ToolHandler for CreateNarrativeTool<S> {
    fn tool_info() -> ToolInfo {
        ToolInfo::new(
            "create_narrative",
            Some("Create a narrative from TOML content. Returns narrative metadata.".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "toml_content": {
                        "type": "string",
                        "description": "TOML narrative definition"
                    },
                    "name": {
                        "type": "string",
                        "description": "Optional name override"
                    }
                },
                "required": ["toml_content"]
            }),
        )
    }

    #[tracing::instrument(skip(self, args), fields(tool = "create_narrative"))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        tracing::debug!("Creating narrative from TOML");

        let toml_content = args
            .get("toml_content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing toml_content parameter".to_string(),
                ))
            })?;

        let name_override = args
            .get("name")
            .and_then(|v| v.as_str());

        match self.storage.parse_narrative(toml_content, name_override).await {
            Ok(narrative_data) => {
                let result = json!({
                    "success": true,
                    "narrative": narrative_data
                });

                tracing::debug!("Narrative created successfully");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to parse narrative");
                Err(McpClientError::new(McpClientErrorKind::ToolExecutionFailed(
                    format!("Parse error: {}", e),
                )))
            }
        }
    }
}

/// Tool for listing available narratives.
///
/// Lists narratives from configured directories.
pub struct ListNarrativesTool<S: botticelli_interface::NarrativeStorageOperations> {
    storage: S,
}

impl<S: botticelli_interface::NarrativeStorageOperations> ListNarrativesTool<S> {
    /// Create new list narratives tool.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl<S: botticelli_interface::NarrativeStorageOperations> ToolHandler for ListNarrativesTool<S> {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "list_narratives",
            Some("List available narrative files in the narratives directory.".to_string()),
            json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Optional glob pattern to filter narratives (e.g., '*.toml')"
                    }
                }
            }),
        )
    }

    #[tracing::instrument(skip(self, args), fields(tool = "list_narratives"))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        tracing::debug!("Listing narratives");

        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str());

        match self.storage.list_narratives(pattern).await {
            Ok(narratives) => {
                let result = json!({
                    "success": true,
                    "pattern": pattern.unwrap_or("*.toml"),
                    "count": narratives.len(),
                    "narratives": narratives
                });

                tracing::debug!(count = narratives.len(), "Listed narratives");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to list narratives");
                Err(McpClientError::new(McpClientErrorKind::ToolExecutionFailed(
                    format!("List error: {}", e),
                )))
            }
        }
    }
}

/// Tool for loading narratives from file.
///
/// Reads TOML file and validates narrative structure.
pub struct LoadNarrativeTool<S: botticelli_interface::NarrativeStorageOperations> {
    storage: S,
}

impl<S: botticelli_interface::NarrativeStorageOperations> LoadNarrativeTool<S> {
    /// Create new load narrative tool.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl<S: botticelli_interface::NarrativeStorageOperations> ToolHandler for LoadNarrativeTool<S> {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "load_narrative",
            Some(
                "Load a narrative from file. Returns narrative metadata and structure.".to_string(),
            ),
            json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "Narrative filename (e.g., 'example.toml')"
                    }
                },
                "required": ["filename"]
            }),
        )
    }

    #[tracing::instrument(skip(self, args), fields(tool = "load_narrative"))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        tracing::debug!("Loading narrative from file");

        let filename = args
            .get("filename")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing filename parameter".to_string(),
                ))
            })?;

        match self.storage.load_narrative(filename).await {
            Ok(narrative_data) => {
                let result = json!({
                    "success": true,
                    "file": filename,
                    "narrative": narrative_data
                });

                tracing::debug!(file = %filename, "Narrative loaded successfully");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
            Err(e) => {
                tracing::error!(error = ?e, file = %filename, "Failed to load narrative");
                Err(McpClientError::new(McpClientErrorKind::ToolExecutionFailed(
                    format!("Load error: {}", e),
                )))
            }
        }
    }
}

/// Tool for validating narrative structure.
///
/// Validates TOML syntax, required fields, and act references.
pub struct ValidateNarrativeTool<S: botticelli_interface::NarrativeStorageOperations> {
    storage: S,
}

impl<S: botticelli_interface::NarrativeStorageOperations> ValidateNarrativeTool<S> {
    /// Create new validate narrative tool.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl<S: botticelli_interface::NarrativeStorageOperations> ToolHandler for ValidateNarrativeTool<S> {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "validate_narrative",
            Some(
                "Validate narrative TOML structure and references. Returns validation results."
                    .to_string(),
            ),
            json!({
                "type": "object",
                "properties": {
                    "toml_content": {
                        "type": "string",
                        "description": "TOML narrative content to validate"
                    }
                },
                "required": ["toml_content"]
            }),
        )
    }

    #[tracing::instrument(skip(self, args), fields(tool = "validate_narrative"))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        tracing::debug!("Validating narrative");

        let toml_content = args
            .get("toml_content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing toml_content parameter".to_string(),
                ))
            })?;

        match self.storage.validate_narrative(toml_content).await {
            Ok(validation_result) => {
                tracing::debug!("Validation complete");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&validation_result)
                        .unwrap_or_else(|_| validation_result.to_string()),
                }])
            }
            Err(e) => {
                let result = json!({
                    "valid": false,
                    "error": format!("{}", e),
                    "error_type": "parse_error"
                });

                tracing::error!(error = ?e, "Validation failed");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_narrative_tool() {
        let tool = CreateNarrativeTool;
        let info = tool.tool_info();

        assert_eq!(info.name, "create_narrative");
        assert!(info.description.is_some());
    }

    #[tokio::test]
    async fn test_validate_narrative_tool() {
        let tool = ValidateNarrativeTool;
        let info = tool.tool_info();

        assert_eq!(info.name, "validate_narrative");
        assert!(info.description.is_some());
    }

    #[tokio::test]
    async fn test_list_narratives_tool() {
        let tool = ListNarrativesTool::new("/tmp");
        let info = tool.tool_info();

        assert_eq!(info.name, "list_narratives");
        assert!(info.description.is_some());
    }

    #[tokio::test]
    async fn test_load_narrative_tool() {
        let tool = LoadNarrativeTool::new("/tmp");
        let info = tool.tool_info();

        assert_eq!(info.name, "load_narrative");
        assert!(info.description.is_some());
    }
}
