//! Helpers for modifying existing narratives based on natural language instructions.

use botticelli_error::{McpError, McpResult};
use tracing::instrument;

/// Apply modification to narrative TOML.
#[instrument(skip(toml))]
pub(crate) fn apply_modification(
    toml: &str,
    modification: &str,
) -> McpResult<(String, Vec<String>)> {
    let lower_mod = modification.to_lowercase();
    let mut changes = Vec::new();

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

    if lower_mod.contains("change model")
        || lower_mod.contains("use model")
        || lower_mod.contains("set model")
        || lower_mod.contains("use gemini")
        || lower_mod.contains("use claude")
        || lower_mod.contains("use gpt")
    {
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

    Err(McpError::invalid_input(format!(
        "Could not understand modification: '{}'. \
         Supported: add/remove act, change model, set temperature, add bot command",
        modification
    )))
}

/// Add an act to the narrative.
#[instrument(skip(toml))]
fn add_act(toml: &str, modification: &str) -> McpResult<(String, String)> {
    let lower_mod = modification.to_lowercase();
    let desc = if let Some(idx) = lower_mod.find("add act") {
        let after_add_act = &modification[idx + "add act".len()..];
        let trimmed = after_add_act.trim();
        if trimmed.starts_with("that") || trimmed.starts_with("which") {
            trimmed
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            trimmed.to_string()
        }
    } else {
        modification.trim().to_string()
    };

    let act_name = extract_act_name_from_mod(&desc);
    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    let toc_idx = lines
        .iter()
        .position(|line| line.trim().starts_with("order = ["))
        .ok_or_else(|| McpError::execution_failed("No [toc] section found".to_string()))?;

    let toc_line = &lines[toc_idx];
    if toc_line.contains(']') {
        let updated_toc = toc_line.replace(']', &format!(", \"{}\"]", act_name));
        lines[toc_idx] = updated_toc;
    }

    let acts_idx = lines
        .iter()
        .position(|line| line.trim() == "[acts]")
        .ok_or_else(|| McpError::execution_failed("No [acts] section found".to_string()))?;

    lines.insert(
        acts_idx + 1,
        format!("{} = \"{}\"", act_name, escape_toml_string(&desc)),
    );

    Ok((lines.join("\n"), format!("Added act '{}'", act_name)))
}

/// Remove an act from the narrative.
#[instrument(skip(toml))]
fn remove_act(toml: &str, modification: &str) -> McpResult<(String, String)> {
    let act_name = extract_act_name_from_mod(modification);
    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    for line in &mut lines {
        if line.trim().starts_with("order = [") {
            *line = line.replace(&format!("\"{}\", ", act_name), "");
            *line = line.replace(&format!(", \"{}\"", act_name), "");
            *line = line.replace(&format!("\"{}\"", act_name), "");
        }
    }

    lines.retain(|line| !line.trim().starts_with(&format!("{} = ", act_name)));

    Ok((lines.join("\n"), format!("Removed act '{}'", act_name)))
}

/// Change the model in the narrative.
#[instrument(skip(toml))]
fn change_model(toml: &str, modification: &str) -> McpResult<(String, String)> {
    let model = extract_model_name(modification)?;
    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    let narrative_idx = lines
        .iter()
        .position(|line| line.trim() == "[narrative]")
        .ok_or_else(|| McpError::execution_failed("No [narrative] section found".to_string()))?;

    let mut found_model = false;
    for i in (narrative_idx + 1)..lines.len() {
        if lines[i].trim().starts_with("model = ") {
            lines[i] = format!("model = \"{}\"", model);
            found_model = true;
            break;
        }
        if lines[i].trim().starts_with('[') {
            lines.insert(i, format!("model = \"{}\"", model));
            found_model = true;
            break;
        }
    }

    if !found_model {
        lines.insert(narrative_idx + 1, format!("model = \"{}\"", model));
    }

    Ok((lines.join("\n"), format!("Changed model to '{}'", model)))
}

/// Change the temperature in the narrative.
#[instrument(skip(toml))]
fn change_temperature(toml: &str, modification: &str) -> McpResult<(String, String)> {
    let temp = extract_temperature(modification)?;
    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    let narrative_idx = lines
        .iter()
        .position(|line| line.trim() == "[narrative]")
        .ok_or_else(|| McpError::execution_failed("No [narrative] section found".to_string()))?;

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

    Ok((lines.join("\n"), format!("Changed temperature to {}", temp)))
}

/// Add a bot command to the narrative.
#[instrument(skip(toml))]
fn add_bot_command(toml: &str, _modification: &str) -> McpResult<(String, String)> {
    let bot_name = "bot_command";
    let platform = "discord";
    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    let insert_idx = lines
        .iter()
        .position(|line| line.trim() == "[toc]")
        .unwrap_or(lines.len());

    lines.insert(insert_idx, format!("\n[bots.{}]", bot_name));
    lines.insert(insert_idx + 1, format!("platform = \"{}\"", platform));
    lines.insert(insert_idx + 2, "command = \"server.get_stats\"".to_string());
    lines.insert(insert_idx + 3, String::new());

    Ok((
        lines.join("\n"),
        format!("Added bot command 'bots.{}'", bot_name),
    ))
}

/// Extract act name from modification text.
#[instrument]
fn extract_act_name_from_mod(text: &str) -> String {
    text.split_whitespace()
        .next()
        .map(|w| w.to_lowercase())
        .unwrap_or_else(|| "new_act".to_string())
}

/// Extract model name from modification text.
#[instrument]
fn extract_model_name(text: &str) -> McpResult<String> {
    let lower = text.to_lowercase();
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
    Err(McpError::invalid_input(
        "Could not identify model name in modification".to_string(),
    ))
}

/// Extract temperature from modification text.
#[instrument]
fn extract_temperature(text: &str) -> McpResult<f64> {
    for word in text.split_whitespace() {
        if let Ok(temp) = word.trim().parse::<f64>()
            && (0.0..=1.0).contains(&temp) {
                return Ok(temp);
            }
    }
    Err(McpError::invalid_input(
        "Could not extract temperature value (must be 0.0-1.0)".to_string(),
    ))
}

/// Escape string for TOML.
#[instrument]
fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
