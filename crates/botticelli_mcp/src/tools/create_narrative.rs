//! Tool for generating narratives from natural language descriptions.

use crate::NarrativeHelper;
use crate::tools::narrative_validation_helpers::{
    add_helpful_comments, auto_fix_common_issues, format_toml, format_validation_result,
};
use crate::tools::McpTool;
use botticelli_error::{McpError, McpResult};
use async_trait::async_trait;
use botticelli_narrative::validator::validate_narrative_toml;
use serde_json::{json, Value};
use tracing::{debug, instrument};

/// Tool for creating narratives from natural language descriptions.
///
/// Generates complete, valid narrative TOML files that can be executed
/// immediately or refined through further modifications.
pub struct CreateNarrativeTool;

#[async_trait]
impl McpTool for CreateNarrativeTool {
    fn name(&self) -> &str {
        "create_narrative"
    }

    fn description(&self) -> &str {
        "Generate a complete narrative TOML from a natural language description. \
         Returns valid, executable narrative ready for use or further refinement. \
         The generated narrative includes [narrative], [toc], and [acts] sections."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Natural language description of the narrative workflow"
                },
                "name": {
                    "type": "string",
                    "description": "Narrative name (alphanumeric + underscores)"
                },
                "default_model": {
                    "type": "string",
                    "description": "Optional default model for all acts"
                },
                "default_temperature": {
                    "type": "number",
                    "description": "Optional default temperature (0.0-1.0)",
                    "minimum": 0.0,
                    "maximum": 1.0
                }
            },
            "required": ["description", "name"]
        })
    }

    #[instrument(skip(self))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        debug!("Creating narrative from description");

        // Extract inputs
        let description = input
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'description'".to_string()))?;

        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'name'".to_string()))?;

        let default_model = input.get("default_model").and_then(|v| v.as_str());

        let default_temperature = input.get("default_temperature").and_then(|v| v.as_f64());

        // Generate narrative TOML
        let mut toml = generate_narrative_toml(description, name, default_model, default_temperature)?;

        // Auto-fix common issues
        let (fixed_toml, fixes_applied) = auto_fix_common_issues(&toml);
        toml = fixed_toml;

        // Format TOML
        toml = format_toml(&toml);

        // Add helpful comments
        let toml_with_comments = add_helpful_comments(&toml);

        // Validate
        let validation = validate_narrative_toml(&toml);

        debug!(
            valid = validation.is_valid(),
            errors = validation.errors.len(),
            warnings = validation.warnings.len(),
            fixes_applied = fixes_applied.len(),
            "Narrative generated and validated"
        );

        // Format validation results with enhanced information
        let validation_json = format_validation_result(&validation);

        // Generate summary
        let act_count = NarrativeHelper::count_acts(&toml);
        let summary = if validation.is_valid() {
            if fixes_applied.is_empty() {
                format!("✅ Created narrative '{}' with {} act(s)", name, act_count)
            } else {
                format!(
                    "✅ Created narrative '{}' with {} act(s) ({} auto-fixes applied)",
                    name,
                    act_count,
                    fixes_applied.len()
                )
            }
        } else {
            format!(
                "❌ Generated narrative has {} error(s) - see validation for details",
                validation.errors.len()
            )
        };

        Ok(json!({
            "toml": toml,
            "toml_with_comments": toml_with_comments,
            "validation": validation_json,
            "summary": summary,
            "auto_fixes_applied": fixes_applied,
            "act_count": act_count
        }))
    }
}

/// Generate narrative TOML from description.
fn generate_narrative_toml(
    description: &str,
    name: &str,
    default_model: Option<&str>,
    default_temperature: Option<f64>,
) -> McpResult<String> {
    // Parse description to extract workflow steps
    let acts = NarrativeHelper::extract_acts_from_description(description);

    // Build TOML
    let mut toml = String::new();

    // [narrative] section
    toml.push_str("[narrative]\n");
    toml.push_str(&format!("name = \"{}\"\n", name));
    toml.push_str(&format!(
        "description = \"{}\"\n",
        NarrativeHelper::escape_toml_string(description)
    ));

    if let Some(model) = default_model {
        toml.push_str(&format!("model = \"{}\"\n", model));
    }

    if let Some(temp) = default_temperature {
        toml.push_str(&format!("temperature = {}\n", temp));
    }

    toml.push('\n');

    // [toc] section
    toml.push_str("[toc]\n");
    toml.push_str("order = [");
    for (i, act) in acts.iter().enumerate() {
        if i > 0 {
            toml.push_str(", ");
        }
        toml.push_str(&format!("\"{}\"", act.name));
    }
    toml.push_str("]\n\n");

    // [acts] section
    toml.push_str("[acts]\n");
    for act in &acts {
        toml.push_str(&format!(
            "{} = \"{}\"\n",
            act.name,
            NarrativeHelper::escape_toml_string(&act.prompt)
        ));
    }

    Ok(toml)
}

