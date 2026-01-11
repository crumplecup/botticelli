//! Narrative TOML validation tool for MCP.
//!
//! DEPRECATED: This module contains the legacy McpTool implementation.
//! New code should use the rmcp-based implementation in `src/rmcp_server.rs`
//! and the types in `src/validate_narrative.rs`.

use crate::tools::McpTool;
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use botticelli_narrative::validator::{ValidationConfig, Validator};
use serde_json::{Value, json};
use std::path::PathBuf;

/// Tool for validating narrative TOML files.
///
/// DEPRECATED: Use the rmcp-based `BotticelliServer::validate_narrative` instead.
/// See `src/rmcp_server.rs` for the new implementation.
///
/// Provides comprehensive validation with actionable error messages,
/// catching common syntax errors and providing fix suggestions.
#[deprecated(
    since = "0.1.0",
    note = "Use rmcp-based BotticelliServer::validate_narrative instead"
)]
pub struct ValidateNarrativeTool;

#[async_trait]
impl McpTool for ValidateNarrativeTool {
    fn name(&self) -> &str {
        "validate_narrative"
    }

    fn description(&self) -> &str {
        "Validate a narrative TOML file or string. Checks syntax, structure, references, \
         model names, and circular dependencies. Returns detailed errors and suggestions."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "TOML content to validate (either this or file_path must be provided)"
                },
                "file_path": {
                    "type": "string",
                    "description": "Path to TOML file to validate (either this or content must be provided)"
                },
                "validate_files": {
                    "type": "boolean",
                    "description": "Check that media and nested narrative files exist",
                    "default": false
                },
                "validate_models": {
                    "type": "boolean",
                    "description": "Warn on unknown model names",
                    "default": true
                },
                "warn_unused": {
                    "type": "boolean",
                    "description": "Warn about unused resources (bots, tables, media)",
                    "default": true
                },
                "strict": {
                    "type": "boolean",
                    "description": "Treat warnings as errors",
                    "default": false
                }
            },
            "oneOf": [
                { "required": ["content"] },
                { "required": ["file_path"] }
            ]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let content = input.get("content").and_then(|v| v.as_str());
        let file_path = input.get("file_path").and_then(|v| v.as_str());
        let validate_files = input
            .get("validate_files")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let validate_models = input
            .get("validate_models")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let warn_unused = input
            .get("warn_unused")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let strict = input
            .get("strict")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Get TOML content
        let toml_content = if let Some(c) = content {
            c.to_string()
        } else if let Some(path) = file_path {
            std::fs::read_to_string(path).map_err(|e| {
                McpError::invalid_input(format!("Failed to read file '{}': {}", path, e))
            })?
        } else {
            return Err(McpError::invalid_input(
                "Either 'content' or 'file_path' must be provided".to_string(),
            ));
        };

        // Configure validation
        let mut config = ValidationConfig::default();
        config = config
            .with_validate_nested_narratives(validate_files)
            .with_validate_media_files(validate_files)
            .with_warn_unknown_models(validate_models)
            .with_warn_unused_resources(warn_unused);

        if let Some(p) =
            file_path.and_then(|p| PathBuf::from(p).parent().map(|parent| parent.to_path_buf()))
        {
            config = config.with_base_dir(Some(p));
        }

        // Validate
        let result = Validator::validate_toml_with_config(&toml_content, &config);

        // Format errors
        let errors: Vec<Value> = result
            .errors()
            .iter()
            .map(|e| {
                json!({
                    "kind": format!("{:?}", e.kind()),
                    "message": e.message(),
                    "suggestion": e.suggestion(),
                    "location": e.location().as_ref().map(|loc| json!({
                        "line": loc.line(),
                        "column": loc.column(),
                        "section": loc.section(),
                    })),
                })
            })
            .collect();

        // Format warnings
        let warnings: Vec<Value> = result
            .warnings()
            .iter()
            .map(|w| {
                json!({
                    "kind": format!("{:?}", w.kind()),
                    "message": w.message(),
                    "location": w.location().as_ref().map(|loc| json!({
                        "line": loc.line(),
                        "column": loc.column(),
                        "section": loc.section(),
                    })),
                })
            })
            .collect();

        let is_valid = result.is_valid();
        let has_warnings = !result.warnings().is_empty();

        Ok(json!({
            "valid": is_valid && (!strict || !has_warnings),
            "errors": errors,
            "warnings": warnings,
            "summary": format!(
                "{} error(s), {} warning(s)",
                errors.len(),
                warnings.len()
            )
        }))
    }
}
