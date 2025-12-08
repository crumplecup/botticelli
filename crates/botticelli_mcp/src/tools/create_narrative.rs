//! Tool for generating narratives from natural language descriptions.

use crate::tools::narrative_validation_helpers::{
    add_helpful_comments, auto_fix_common_issues, format_toml, format_validation_result,
};
use crate::tools::McpTool;
use crate::{McpError, McpResult};
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
            .ok_or_else(|| McpError::InvalidInput("Missing 'description'".to_string()))?;

        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'name'".to_string()))?;

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
        let act_count = count_acts(&toml);
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
    let acts = extract_acts_from_description(description);

    // Build TOML
    let mut toml = String::new();

    // [narrative] section
    toml.push_str("[narrative]\n");
    toml.push_str(&format!("name = \"{}\"\n", name));
    toml.push_str(&format!("description = \"{}\"\n", escape_toml_string(description)));

    if let Some(model) = default_model {
        toml.push_str(&format!("model = \"{}\"\n", model));
    }

    if let Some(temp) = default_temperature {
        toml.push_str(&format!("temperature = {}\n", temp));
    }

    toml.push_str("\n");

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
        toml.push_str(&format!("{} = \"{}\"\n", act.name, escape_toml_string(&act.prompt)));
    }

    Ok(toml)
}

/// Extract acts from natural language description.
fn extract_acts_from_description(description: &str) -> Vec<Act> {
    // Simple heuristic: split on common connectors
    let parts: Vec<&str> = description
        .split(|c| c == ',' || c == ';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        // Fallback: single act
        return vec![Act {
            name: "process".to_string(),
            prompt: description.to_string(),
        }];
    }

    // Look for action verbs and create acts
    let mut acts = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let name = extract_act_name(part, i);
        acts.push(Act {
            name,
            prompt: part.to_string(),
        });
    }

    // Look for "then" patterns
    if description.contains(" then ") {
        let then_parts: Vec<&str> = description.split(" then ").collect();
        if then_parts.len() > 1 {
            acts.clear();
            for (i, part) in then_parts.iter().enumerate() {
                let name = extract_act_name(part, i);
                acts.push(Act {
                    name,
                    prompt: part.trim().to_string(),
                });
            }
        }
    }

    // Look for "and" patterns
    if acts.len() == 1 && description.contains(" and ") {
        let and_parts: Vec<&str> = description.split(" and ").collect();
        if and_parts.len() > 1 {
            acts.clear();
            for (i, part) in and_parts.iter().enumerate() {
                let name = extract_act_name(part, i);
                acts.push(Act {
                    name,
                    prompt: part.trim().to_string(),
                });
            }
        }
    }

    if acts.is_empty() {
        acts.push(Act {
            name: "process".to_string(),
            prompt: description.to_string(),
        });
    }

    acts
}

/// Extract act name from description part.
fn extract_act_name(description: &str, fallback_index: usize) -> String {
    // Common action verbs
    let verbs = [
        "fetch", "get", "retrieve", "load", "read", "analyze", "process", "transform", "generate",
        "create", "build", "format", "send", "post", "publish", "write", "save", "store",
        "compare", "evaluate", "rank", "filter", "select", "choose", "extract", "parse",
    ];

    let lower = description.to_lowercase();

    // Find first verb
    for verb in &verbs {
        if lower.starts_with(verb) || lower.contains(&format!(" {} ", verb)) {
            return verb.to_string();
        }
    }

    // Fallback
    format!("act{}", fallback_index + 1)
}

/// Count acts in TOML.
fn count_acts(toml: &str) -> usize {
    toml.lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('[') && trimmed.contains("acts.")
        })
        .count()
        .max(
            toml.lines()
                .find(|line| line.trim() == "[acts]")
                .map(|_| {
                    toml.lines()
                        .skip_while(|line| !line.trim().starts_with("[acts]"))
                        .skip(1)
                        .take_while(|line| !line.trim().starts_with('['))
                        .filter(|line| line.contains('='))
                        .count()
                })
                .unwrap_or(0),
        )
}

/// Escape string for TOML.
fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Act definition.
struct Act {
    name: String,
    prompt: String,
}
