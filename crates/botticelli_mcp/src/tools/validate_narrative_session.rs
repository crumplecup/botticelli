use crate::tools::{
    ApplyValidationFixesInput, McpTool, PartialNarrativeRegistry, ValidateNarrativeInput,
};
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use serde_json::{Value, json};
use std::sync::Arc;

/// Tool for validating a narrative elicitation session.
pub struct ValidateNarrativeSessionTool {
    registry: Arc<PartialNarrativeRegistry>,
}

impl ValidateNarrativeSessionTool {
    /// Creates a new validate narrative session tool.
    pub fn new(registry: Arc<PartialNarrativeRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl McpTool for ValidateNarrativeSessionTool {
    fn name(&self) -> &str {
        "validate_narrative_session"
    }

    fn description(&self) -> &str {
        "Validate a narrative elicitation session for completeness and correctness. \
         Returns detailed errors, warnings, and completeness report."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {
                    "type": "string",
                    "description": "UUID of the narrative session"
                },
                "strict": {
                    "type": "boolean",
                    "description": "Enable strict validation mode (default: false)",
                    "default": false
                }
            },
            "required": ["narrative_id"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let input: ValidateNarrativeInput =
            serde_json::from_value(input).map_err(|e| McpError::invalid_input(e.to_string()))?;
        let output = crate::tools::elicitation::validation::validate_narrative(
            self.registry.as_ref(),
            input,
        )
        .await?;
        serde_json::to_value(output).map_err(|e| McpError::execution_failed(e.to_string()))
    }
}

/// Tool for applying automated fixes to narrative validation issues.
pub struct ApplyValidationFixesTool {
    registry: Arc<PartialNarrativeRegistry>,
}

impl ApplyValidationFixesTool {
    /// Creates a new apply validation fixes tool.
    pub fn new(registry: Arc<PartialNarrativeRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl McpTool for ApplyValidationFixesTool {
    fn name(&self) -> &str {
        "apply_validation_fixes"
    }

    fn description(&self) -> &str {
        "Apply automated fixes to resolve validation issues in a narrative session. \
         Can fix missing defaults and other auto-fixable problems."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {
                    "type": "string",
                    "description": "UUID of the narrative session"
                },
                "fix_types": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Types of fixes to apply (e.g., 'missing_defaults', 'all')"
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Confirm application of fixes (default: false)",
                    "default": false
                }
            },
            "required": ["narrative_id", "fix_types"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let input: ApplyValidationFixesInput =
            serde_json::from_value(input).map_err(|e| McpError::invalid_input(e.to_string()))?;
        let output = crate::tools::elicitation::validation::apply_validation_fixes(
            self.registry.as_ref(),
            input,
        )
        .await?;
        serde_json::to_value(output).map_err(|e| McpError::execution_failed(e.to_string()))
    }
}
