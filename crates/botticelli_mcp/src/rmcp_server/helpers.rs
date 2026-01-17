//! Helper functions for MCP server operations.

use crate::tools::narrative_validation_helpers::auto_fix_common_issues;
use botticelli_narrative::validator::Validator;
use std::path::Path;
use tracing::{error, instrument};

/// Helper function to convert any error to rmcp::ErrorData while preserving error details in logs.
///
/// This logs the full error with context before converting to ErrorData, ensuring we don't
/// lose debugging information even though rmcp::ErrorData only stores a message.
#[instrument(skip(err), fields(error_type = std::any::type_name::<E>()))]
pub(super) fn to_mcp_error<E: std::fmt::Display + std::fmt::Debug>(
    err: E,
    context: &str,
) -> rmcp::ErrorData {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    // Log the full error with debug representation
    error!(error = ?err, context = context, "Error occurred");

    // Convert to ErrorData with context
    rmcp::ErrorData::new(
        ErrorCode::INTERNAL_ERROR,
        Cow::Owned(format!("{}: {}", context, err)),
        None,
    )
}

/// Get default model name.
#[instrument]
pub(super) fn default_model() -> String {
    "gemini-2.0-flash-exp".to_string()
}

// Helper functions for narrative generation

/// Generate narrative TOML from description.
#[instrument(skip(description), fields(description_len = description.len()))]
pub(super) fn generate_narrative_toml(
    description: &str,
    name: &str,
    default_model: Option<&str>,
    default_temperature: Option<f64>,
) -> Result<String, rmcp::ErrorData> {
    
    

    let model = default_model.unwrap_or("gemini-2.0-flash-exp");
    let temperature = default_temperature.unwrap_or(1.0);

    // Detect if description mentions specific steps/acts
    let has_steps = description.to_lowercase().contains("step")
        || description.to_lowercase().contains("then")
        || description.to_lowercase().contains("first");

    let toml = if has_steps {
        // Multi-act narrative
        format!(
            r#"# {name}
# {description}

[narrative]
name = "{name}"
description = "{description}"
model = "{model}"
temperature = {temperature}

[toc]
order = ["act1", "act2", "act3"]

[acts]
act1 = "Complete first step: [describe step 1]"
act2 = "Complete second step: [describe step 2]"
act3 = "Complete final step: [describe step 3]"
"#
        )
    } else {
        // Single-act narrative
        format!(
            r#"# {name}
# {description}

[narrative]
name = "{name}"
description = "{description}"
model = "{model}"
temperature = {temperature}

[toc]
order = ["main"]

[acts]
main = "{description}"
"#
        )
    };

    // Validate the generated TOML
    let _path = Path::new(&format!("{}.toml", name));

    match Validator::validate_toml(&toml) {
        validation if validation.is_valid() => Ok(toml),
        _validation => {
            // Try auto-fixing if validation failed
            let (fixed, _fixes) = auto_fix_common_issues(&toml);
            // Return the fixed version (it might still have issues, but it's better)
            Ok(fixed)
        }
    }
}

/// Apply a modification to narrative TOML.
#[instrument(skip(toml), fields(toml_len = toml.len()))]
pub(super) fn apply_modification(
    toml: &str,
    modification: &str,
) -> Result<(String, String), rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    let lower = modification.to_lowercase();

    // Detect modification type
    if lower.contains("add") && (lower.contains("act") || lower.contains("step")) {
        add_act(toml, modification)
    } else if lower.contains("remove") && (lower.contains("act") || lower.contains("step")) {
        remove_act(toml, modification)
    } else if lower.contains("model") || lower.contains("llm") {
        change_model(toml, modification)
    } else if lower.contains("temperature") || lower.contains("temp") {
        change_temperature(toml, modification)
    } else if lower.contains("bot") && lower.contains("command") {
        add_bot_command(toml)
    } else {
        Err(rmcp::ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            Cow::Borrowed("Could not determine modification type. Try: 'add act', 'remove act', 'change model', 'set temperature', or 'add bot command'"),
            None,
        ))
    }
}

/// Add an act to the narrative.
#[instrument(skip(toml), fields(toml_len = toml.len()))]
fn add_act(toml: &str, modification: &str) -> Result<(String, String), rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    let act_name = extract_act_name_from_mod(modification);

    // Find [acts] section and [toc] section
    let lines: Vec<&str> = toml.lines().collect();
    let mut acts_start = None;
    let mut toc_start = None;
    
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "[acts]" {
            acts_start = Some(i);
        }
        if line.trim() == "[toc]" {
            toc_start = Some(i);
        }
    }

    let acts_start = acts_start.ok_or_else(|| {
        rmcp::ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            Cow::Borrowed("Could not find [acts] section in TOML"),
            None,
        )
    })?;

    // Find end of [acts] section (next section or EOF)
    let mut acts_end = lines.len();
    for i in (acts_start + 1)..lines.len() {
        if lines[i].starts_with('[') && !lines[i].starts_with("# [") {
            acts_end = i;
            break;
        }
    }

    // Insert new act before the end of acts section
    let new_act = format!("{} = \"[Describe what this act should do]\"", act_name);
    
    let mut result = Vec::new();
    result.extend_from_slice(&lines[..acts_end]);
    result.push(new_act.as_str());
    result.extend_from_slice(&lines[acts_end..]);

    // Update [toc] order if present
    let mut modified = result.join("\n");
    if let Some(toc_idx) = toc_start {
        // Find the order line
        for i in (toc_idx + 1)..result.len() {
            if result[i].trim().starts_with("order = ") {
                // Extract existing order
                let order_line = result[i].trim();
                if let Some(start) = order_line.find('[') {
                    if let Some(end) = order_line.find(']') {
                        let current_acts = &order_line[start + 1..end];
                        let new_order = if current_acts.trim().is_empty() {
                            format!("order = [\"{}\"]", act_name)
                        } else {
                            format!("order = [{}, \"{}\"]", current_acts, act_name)
                        };
                        result[i] = Box::leak(new_order.into_boxed_str());
                        modified = result.join("\n");
                        break;
                    }
                }
            }
        }
    }

    let change = format!("Added act '{}'", act_name);
    Ok((modified, change))
}

/// Remove an act from the narrative.
#[instrument(skip(toml))]
fn remove_act(toml: &str, modification: &str) -> Result<(String, String), rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    let act_name = extract_act_name_from_mod(modification);

    let lines: Vec<&str> = toml.lines().collect();
    let mut result = Vec::new();
    let mut removed = false;
    let mut in_acts = false;
    let mut in_toc = false;

    for line in lines {
        if line.trim() == "[acts]" {
            in_acts = true;
            in_toc = false;
            result.push(line);
            continue;
        }
        if line.trim() == "[toc]" {
            in_toc = true;
            in_acts = false;
            result.push(line);
            continue;
        }
        if line.starts_with('[') && !line.starts_with("# [") {
            in_acts = false;
            in_toc = false;
            result.push(line);
            continue;
        }

        // Remove act from [acts] section
        if in_acts && line.trim().starts_with(&format!("{} = ", act_name)) {
            removed = true;
            continue;
        }

        // Remove act from [toc] order
        if in_toc && line.trim().starts_with("order = ") {
            let updated = line.replace(&format!("\"{}\", ", act_name), "")
                .replace(&format!(", \"{}\"", act_name), "")
                .replace(&format!("\"{}\"", act_name), "");
            result.push(Box::leak(updated.into_boxed_str()));
            continue;
        }

        result.push(line);
    }

    if !removed {
        return Err(rmcp::ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            Cow::Owned(format!("Act '{}' not found", act_name)),
            None,
        ));
    }

    let modified = result.join("\n");
    let change = format!("Removed act '{}'", act_name);

    Ok((modified, change))
}

/// Change the model in the narrative.
#[instrument(skip(toml), fields(toml_len = toml.len()))]
fn change_model(toml: &str, modification: &str) -> Result<(String, String), rmcp::ErrorData> {
    let model = extract_model_name(modification)?;

    let lines: Vec<&str> = toml.lines().collect();
    let mut result = Vec::new();
    let mut changed = false;

    for line in lines {
        if line.trim().starts_with("model = ") {
            result.push(format!(r#"model = "{}""#, model));
            changed = true;
        } else {
            result.push(line.to_string());
        }
    }

    if !changed {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;
        return Err(rmcp::ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            Cow::Borrowed("No model field found in TOML"),
            None,
        ));
    }

    let modified = result.join("\n");
    let change = format!("Changed model to '{}'", model);

    Ok((modified, change))
}

/// Change the temperature in the narrative.
#[instrument(skip(toml), fields(toml_len = toml.len()))]
fn change_temperature(
    toml: &str,
    modification: &str,
) -> Result<(String, String), rmcp::ErrorData> {
    let temperature = extract_temperature(modification)?;

    let lines: Vec<&str> = toml.lines().collect();
    let mut result = Vec::new();
    let mut changed = false;

    for line in lines {
        if line.trim().starts_with("temperature = ") {
            result.push(format!("temperature = {}", temperature));
            changed = true;
        } else {
            result.push(line.to_string());
        }
    }

    if !changed {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;
        return Err(rmcp::ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            Cow::Borrowed("No temperature field found in TOML"),
            None,
        ));
    }

    let modified = result.join("\n");
    let change = format!("Changed temperature to {}", temperature);

    Ok((modified, change))
}

/// Add a bot command section.
#[instrument(skip(toml), fields(toml_len = toml.len()))]
fn add_bot_command(toml: &str) -> Result<(String, String), rmcp::ErrorData> {
    let mut lines: Vec<String> = toml.lines().map(String::from).collect();

    // Add bot command at the end
    let bot_name = "example_command";
    let bot_section = format!(
        r#"
[[bot.commands]]
name = "{}"
description = "Example bot command"
narrative = "main"
"#,
        bot_name
    );

    lines.push(bot_section);

    let modified = lines.join("\n");
    let change = format!("Added bot command 'bots.{}'", bot_name);

    Ok((modified, change))
}

/// Extract act name from modification text.
#[instrument]
fn extract_act_name_from_mod(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if let Some(verb) = words.first() {
        return verb.to_lowercase();
    }
    "new_act".to_string()
}

/// Extract model name from modification text.
#[instrument]
fn extract_model_name(text: &str) -> Result<String, rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

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

    Err(rmcp::ErrorData::new(
        ErrorCode::INVALID_PARAMS,
        Cow::Borrowed("Could not identify model name in modification"),
        None,
    ))
}

/// Extract temperature from modification text.
#[instrument]
fn extract_temperature(text: &str) -> Result<f64, rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    let words: Vec<&str> = text.split_whitespace().collect();

    for word in words {
        if let Ok(temp) = word.trim().parse::<f64>() {
            if (0.0..=1.0).contains(&temp) {
                return Ok(temp);
            }
        }
    }

    Err(rmcp::ErrorData::new(
        ErrorCode::INVALID_PARAMS,
        Cow::Borrowed("Could not extract temperature value (must be 0.0-1.0)"),
        None,
    ))
}
