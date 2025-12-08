//! Tool for modifying existing narratives based on natural language instructions.

use crate::tools::narrative_validation_helpers::{
    auto_fix_common_issues, format_toml, format_validation_result,
};
use crate::tools::McpTool;
use crate::{McpError, McpResult};
use async_trait::async_trait;
use botticelli_narrative::validator::validate_narrative_toml;
use serde_json::{json, Value};
use tracing::{debug, instrument};

/// Tool for modifying narratives based on natural language instructions.
///
/// Takes an existing narrative TOML and applies modifications based on
/// natural language descriptions. Returns updated, valid TOML.
pub struct ModifyNarrativeTool;

#[async_trait]
impl McpTool for ModifyNarrativeTool {
    fn name(&self) -> &str {
        "modify_narrative"
    }

    fn description(&self) -> &str {
        "Modify an existing narrative based on natural language instructions. \
         Can add/remove/modify acts, change models, add resources, or update metadata. \
         Returns the complete updated narrative with validation results."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_toml": {
                    "type": "string",
                    "description": "Existing narrative TOML to modify"
                },
                "modification": {
                    "type": "string",
                    "description": "Natural language description of the modification"
                },
                "save_to": {
                    "type": "string",
                    "description": "Optional path to save modified narrative"
                }
            },
            "required": ["narrative_toml", "modification"]
        })
    }

    #[instrument(skip(self, input))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        debug!("Modifying narrative");

        // Extract inputs
        let narrative_toml = input
            .get("narrative_toml")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'narrative_toml'".to_string()))?;

        let modification = input
            .get("modification")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'modification'".to_string()))?;

        let save_to = input.get("save_to").and_then(|v| v.as_str());

        // Apply modification
        let (mut modified_toml, mut changes) = apply_modification(narrative_toml, modification)?;

        // Auto-fix common issues
        let (fixed_toml, fixes_applied) = auto_fix_common_issues(&modified_toml);
        if !fixes_applied.is_empty() {
            modified_toml = fixed_toml;
            changes.extend(fixes_applied.iter().map(|f| format!("Auto-fix: {}", f)));
        }

        // Format TOML
        modified_toml = format_toml(&modified_toml);

        // Validate
        let validation = validate_narrative_toml(&modified_toml);

        debug!(
            valid = validation.is_valid(),
            changes = changes.len(),
            auto_fixes = fixes_applied.len(),
            "Narrative modified and validated"
        );

        // Optionally save to file
        let mut saved_to = None;
        if let Some(path) = save_to {
            tokio::fs::write(path, &modified_toml)
                .await
                .map_err(|e| McpError::ToolExecutionFailed(format!("Failed to save file: {}", e)))?;
            saved_to = Some(path.to_string());
            debug!(path, "Saved modified narrative to file");
        }

        // Format validation results with enhanced information
        let validation_json = format_validation_result(&validation);

        Ok(json!({
            "toml": modified_toml,
            "validation": validation_json,
            "changes": changes,
            "saved_to": saved_to
        }))
    }
}

/// Apply modification to narrative TOML.
fn apply_modification(toml: &str, modification: &str) -> McpResult<(String, Vec<String>)> {
    let lower_mod = modification.to_lowercase();
    let mut changes = Vec::new();

    // Detect modification type
    if lower_mod.contains("add act") || lower_mod.contains("add an act") {
        let (modified, change) = add_act(toml, modification)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    if lower_mod.contains("remove act") || lower_mod.contains("delete act") {
        let (modified, change) = remove_act(toml, modification)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    if lower_mod.contains("change model") || lower_mod.contains("use model") || lower_mod.contains("set model") 
        || lower_mod.contains("use gemini") || lower_mod.contains("use claude") || lower_mod.contains("use gpt") {
        let (modified, change) = change_model(toml, modification)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    if lower_mod.contains("temperature") {
        let (modified, change) = change_temperature(toml, modification)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    if lower_mod.contains("add bot") || lower_mod.contains("bot command") {
        let (modified, change) = add_bot_command(toml, modification)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    // Default: try to parse as a general modification
    Err(McpError::InvalidInput(format!(
        "Could not understand modification: '{}'. \
         Supported: add/remove act, change model, set temperature, add bot command",
        modification
    )))
}

/// Add an act to the narrative.
fn add_act(toml: &str, modification: &str) -> McpResult<(String, String)> {
    // Extract act description (everything after "add act" or "Add act")
    let lower_mod = modification.to_lowercase();
    let desc = if let Some(idx) = lower_mod.find("add act") {
        let after_add_act = &modification[idx + "add act".len()..];
        // Skip "that" if present
        let trimmed = after_add_act.trim();
        if trimmed.starts_with("that") || trimmed.starts_with("which") {
            trimmed.split_whitespace().skip(1).collect::<Vec<_>>().join(" ")
        } else {
            trimmed.to_string()
        }
    } else {
        modification.trim().to_string()
    };

    // Generate act name
    let act_name = extract_act_name_from_mod(&desc);

    // Find [toc] and [acts] sections
    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Find TOC line
    let toc_idx = lines
        .iter()
        .position(|line| line.trim().starts_with("order = ["))
        .ok_or_else(|| McpError::ToolExecutionFailed("No [toc] section found".to_string()))?;

    // Add to TOC
    let toc_line = &lines[toc_idx];
    if toc_line.contains(']') {
        let updated_toc = toc_line.replace(']', &format!(", \"{}\"]", act_name));
        lines[toc_idx] = updated_toc;
    }

    // Find [acts] section
    let acts_idx = lines
        .iter()
        .position(|line| line.trim() == "[acts]")
        .ok_or_else(|| McpError::ToolExecutionFailed("No [acts] section found".to_string()))?;

    // Add act definition
    lines.insert(acts_idx + 1, format!("{} = \"{}\"", act_name, escape_toml_string(&desc)));

    let modified = lines.join("\n");
    let change = format!("Added act '{}'", act_name);

    Ok((modified, change))
}

/// Remove an act from the narrative.
fn remove_act(toml: &str, modification: &str) -> McpResult<(String, String)> {
    // Extract act name to remove
    let act_name = extract_act_name_from_mod(modification);

    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Remove from TOC
    for line in &mut lines {
        if line.trim().starts_with("order = [") {
            *line = line.replace(&format!("\"{}\", ", act_name), "");
            *line = line.replace(&format!(", \"{}\"", act_name), "");
            *line = line.replace(&format!("\"{}\"", act_name), "");
        }
    }

    // Remove act definition
    lines.retain(|line| !line.trim().starts_with(&format!("{} = ", act_name)));

    let modified = lines.join("\n");
    let change = format!("Removed act '{}'", act_name);

    Ok((modified, change))
}

/// Change the model in the narrative.
fn change_model(toml: &str, modification: &str) -> McpResult<(String, String)> {
    // Extract model name
    let model = extract_model_name(modification)?;

    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Find [narrative] section
    let narrative_idx = lines
        .iter()
        .position(|line| line.trim() == "[narrative]")
        .ok_or_else(|| McpError::ToolExecutionFailed("No [narrative] section found".to_string()))?;

    // Find or add model line
    let mut found_model = false;
    for i in (narrative_idx + 1)..lines.len() {
        if lines[i].trim().starts_with("model = ") {
            lines[i] = format!("model = \"{}\"", model);
            found_model = true;
            break;
        }
        if lines[i].trim().starts_with('[') {
            // Next section, insert before it
            lines.insert(i, format!("model = \"{}\"", model));
            found_model = true;
            break;
        }
    }

    if !found_model {
        lines.insert(narrative_idx + 1, format!("model = \"{}\"", model));
    }

    let modified = lines.join("\n");
    let change = format!("Changed model to '{}'", model);

    Ok((modified, change))
}

/// Change the temperature in the narrative.
fn change_temperature(toml: &str, modification: &str) -> McpResult<(String, String)> {
    // Extract temperature value
    let temp = extract_temperature(modification)?;

    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Find [narrative] section
    let narrative_idx = lines
        .iter()
        .position(|line| line.trim() == "[narrative]")
        .ok_or_else(|| McpError::ToolExecutionFailed("No [narrative] section found".to_string()))?;

    // Find or add temperature line
    let mut found_temp = false;
    for i in (narrative_idx + 1)..lines.len() {
        if lines[i].trim().starts_with("temperature = ") {
            lines[i] = format!("temperature = {}", temp);
            found_temp = true;
            break;
        }
        if lines[i].trim().starts_with('[') {
            lines.insert(i, format!("temperature = {}", temp));
            found_temp = true;
            break;
        }
    }

    if !found_temp {
        lines.insert(narrative_idx + 1, format!("temperature = {}", temp));
    }

    let modified = lines.join("\n");
    let change = format!("Changed temperature to {}", temp);

    Ok((modified, change))
}

/// Add a bot command to the narrative.
fn add_bot_command(toml: &str, _modification: &str) -> McpResult<(String, String)> {
    // Parse bot command details from modification
    let bot_name = "bot_command"; // Simplified for MVP
    let platform = "discord"; // Default

    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Find where to insert [bots] section (before [toc])
    let insert_idx = lines
        .iter()
        .position(|line| line.trim() == "[toc]")
        .unwrap_or(lines.len());

    // Add bot section
    lines.insert(insert_idx, format!("\n[bots.{}]", bot_name));
    lines.insert(insert_idx + 1, format!("platform = \"{}\"", platform));
    lines.insert(insert_idx + 2, "command = \"server.get_stats\"".to_string());
    lines.insert(insert_idx + 3, String::new());

    let modified = lines.join("\n");
    let change = format!("Added bot command 'bots.{}'", bot_name);

    Ok((modified, change))
}

/// Extract act name from modification text.
fn extract_act_name_from_mod(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if let Some(verb) = words.first() {
        return verb.to_lowercase();
    }
    "new_act".to_string()
}

/// Extract model name from modification text.
fn extract_model_name(text: &str) -> McpResult<String> {
    let lower = text.to_lowercase();

    // Common model patterns
    if lower.contains("claude") {
        return Ok("claude-3-5-sonnet-20241022".to_string());
    }
    if lower.contains("gemini") {
        return Ok("gemini-2.0-flash-exp".to_string());
    }
    if lower.contains("gpt-4") {
        return Ok("gpt-4".to_string());
    }
    if lower.contains("gpt") {
        return Ok("gpt-4-turbo".to_string());
    }

    Err(McpError::InvalidInput(
        "Could not identify model name in modification".to_string(),
    ))
}

/// Extract temperature from modification text.
fn extract_temperature(text: &str) -> McpResult<f64> {
    let words: Vec<&str> = text.split_whitespace().collect();

    for word in words {
        if let Ok(temp) = word.trim().parse::<f64>() {
            if (0.0..=1.0).contains(&temp) {
                return Ok(temp);
            }
        }
    }

    Err(McpError::InvalidInput(
        "Could not extract temperature value (must be 0.0-1.0)".to_string(),
    ))
}

/// Escape string for TOML.
fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
