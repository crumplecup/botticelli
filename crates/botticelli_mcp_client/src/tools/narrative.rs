//! Narrative generation and execution tools.
//!
//! Exposes Botticelli's narrative system as MCP tools for LLM orchestration.

use crate::{McpClientError, McpClientErrorKind, McpClientResult, ToolHandler};
use async_trait::async_trait;
use botticelli_narrative::Narrative;
use pmcp::{Content, ToolInfo};
use serde_json::{json, Value};
use tracing::{debug, error, instrument};

/// Tool for creating narratives from TOML content.
///
/// Parses TOML and validates narrative structure.
pub struct CreateNarrativeTool;

#[async_trait]
impl ToolHandler for CreateNarrativeTool {
    fn tool_info(&self) -> ToolInfo {
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

    #[instrument(skip(self, args), fields(tool = "create_narrative"))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        debug!("Creating narrative from TOML");

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
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match Narrative::from_toml_str(toml_content, name_override.as_deref()) {
            Ok(narrative) => {
                let metadata = narrative.metadata();
                let result = json!({
                    "success": true,
                    "narrative": {
                        "name": name_override.unwrap_or_else(|| metadata.name().to_string()),
                        "description": metadata.description(),
                        "template": metadata.template(),
                        "target": metadata.target(),
                        "act_count": narrative.toc().order().len(),
                        "acts": narrative.toc().order()
                    }
                });

                debug!(name = %metadata.name(), "Narrative created successfully");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
            Err(e) => {
                error!(error = ?e, "Failed to parse narrative");
                Err(McpClientError::new(McpClientErrorKind::InvalidToolCall(format!(
                    "Parse error: {}",
                    e
                ))))
            }
        }
    }
}

/// Tool for listing available narratives.
///
/// Lists narratives from configured directories.
pub struct ListNarrativesTool {
    narratives_dir: String,
}

impl ListNarrativesTool {
    /// Create new list narratives tool.
    pub fn new(narratives_dir: impl Into<String>) -> Self {
        Self {
            narratives_dir: narratives_dir.into(),
        }
    }
}

#[async_trait]
impl ToolHandler for ListNarrativesTool {
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

    #[instrument(skip(self, args), fields(tool = "list_narratives", dir = %self.narratives_dir))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        debug!("Listing narratives");

        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("*.toml");

        let path = std::path::Path::new(&self.narratives_dir);
        if !path.exists() {
            return Err(McpClientError::new(McpClientErrorKind::InvalidToolCall(format!(
                "Narratives directory not found: {}",
                self.narratives_dir
            ))));
        }

        let glob_pattern = format!("{}/{}", self.narratives_dir, pattern);
        match glob::glob(&glob_pattern) {
            Ok(entries) => {
                let narratives: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .collect();

                let result = json!({
                    "success": true,
                    "directory": self.narratives_dir,
                    "pattern": pattern,
                    "count": narratives.len(),
                    "narratives": narratives
                });

                debug!(count = narratives.len(), "Listed narratives");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
            Err(e) => {
                error!(error = ?e, "Failed to list narratives");
                Err(McpClientError::new(McpClientErrorKind::InvalidToolCall(format!(
                    "Glob error: {}",
                    e
                ))))
            }
        }
    }
}

/// Tool for loading narratives from file.
///
/// Reads TOML file and validates narrative structure.
pub struct LoadNarrativeTool {
    narratives_dir: String,
}

impl LoadNarrativeTool {
    /// Create new load narrative tool.
    pub fn new(narratives_dir: impl Into<String>) -> Self {
        Self {
            narratives_dir: narratives_dir.into(),
        }
    }
}

#[async_trait]
impl ToolHandler for LoadNarrativeTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "load_narrative",
            Some("Load a narrative from file. Returns narrative metadata and structure.".to_string()),
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

    #[instrument(skip(self, args), fields(tool = "load_narrative"))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        debug!("Loading narrative from file");

        let filename = args
            .get("filename")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing filename parameter".to_string(),
                ))
            })?;

        let path = std::path::Path::new(&self.narratives_dir).join(filename);
        
        match Narrative::from_file(&path) {
            Ok(narrative) => {
                let metadata = narrative.metadata();
                let result = json!({
                    "success": true,
                    "file": filename,
                    "narrative": {
                        "name": metadata.name(),
                        "description": metadata.description(),
                        "template": metadata.template(),
                        "target": metadata.target(),
                        "model": metadata.model(),
                        "temperature": metadata.temperature(),
                        "max_tokens": metadata.max_tokens(),
                        "act_count": narrative.toc().order().len(),
                        "acts": narrative.toc().order()
                    }
                });

                debug!(name = %metadata.name(), file = %filename, "Narrative loaded successfully");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
            Err(e) => {
                error!(error = ?e, file = %filename, "Failed to load narrative");
                Err(McpClientError::new(McpClientErrorKind::InvalidToolCall(format!(
                    "Load error: {}",
                    e
                ))))
            }
        }
    }
}

/// Tool for validating narrative structure.
///
/// Validates TOML syntax, required fields, and act references.
pub struct ValidateNarrativeTool;

#[async_trait]
impl ToolHandler for ValidateNarrativeTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "validate_narrative",
            Some("Validate narrative TOML structure and references. Returns validation results.".to_string()),
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

    #[instrument(skip(self, args), fields(tool = "validate_narrative"))]
    async fn execute(&self, args: Value) -> McpClientResult<Vec<Content>> {
        debug!("Validating narrative");

        let toml_content = args
            .get("toml_content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpClientError::new(McpClientErrorKind::InvalidToolCall(
                    "Missing toml_content parameter".to_string(),
                ))
            })?;

        match Narrative::from_toml_str(toml_content, None) {
            Ok(narrative) => {
                let metadata = narrative.metadata();
                let toc = narrative.toc();
                
                // Validate all acts exist
                let mut missing_acts = Vec::new();
                for act_name in toc.order() {
                    if narrative.acts().get(act_name).is_none() {
                        missing_acts.push(act_name.clone());
                    }
                }

                let is_valid = missing_acts.is_empty();
                let result = json!({
                    "valid": is_valid,
                    "narrative_name": metadata.name(),
                    "act_count": toc.order().len(),
                    "acts": toc.order(),
                    "missing_acts": missing_acts,
                    "warnings": if !is_valid {
                        vec![format!("Missing act definitions: {:?}", missing_acts)]
                    } else {
                        Vec::<String>::new()
                    }
                });

                debug!(valid = is_valid, name = %metadata.name(), "Validation complete");
                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string()),
                }])
            }
            Err(e) => {
                let result = json!({
                    "valid": false,
                    "error": format!("{}", e),
                    "error_type": "parse_error"
                });

                error!(error = ?e, "Validation failed");
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
