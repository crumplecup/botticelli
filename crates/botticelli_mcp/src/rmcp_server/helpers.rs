//! Helper functions for MCP server operations.

use crate::tools::narrative_validation_helpers::auto_fix_common_issues;
use botticelli_narrative::validator::Validator;
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

    // Convert to ErrorData with context (use Debug format to preserve structure)
    rmcp::ErrorData::new(
        ErrorCode::INTERNAL_ERROR,
        Cow::Owned(format!("{}: {:?}", context, err)),
        None,
    )
}

/// Get default model name.
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
#[instrument]
pub(super) fn default_model() -> String {
    "gemini-2.0-flash-exp".to_string()
}

// Helper functions for narrative generation

/// Generate narrative TOML from description using smart act extraction.
#[instrument(skip(description), fields(description_len = description.len()))]
pub(super) fn generate_narrative_toml(
    description: &str,
    name: &str,
    default_model: Option<&str>,
    default_temperature: Option<f64>,
) -> Result<String, rmcp::ErrorData> {
    use crate::partial::{PartialAct, PartialNarrative};
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;
    use tracing::{debug, warn};

    let model = default_model.unwrap_or("gemini-2.0-flash-exp");
    let temperature = default_temperature.unwrap_or(1.0);

    // Extract acts from description using heuristics
    let acts = extract_acts_from_description(description);

    debug!(
        act_count = acts.len(),
        model = model,
        temperature = temperature,
        "Extracted acts from description"
    );

    // Build narrative using PartialNarrative
    let mut partial = PartialNarrative::default();
    partial
        .with_name(Some(name.to_string()))
        .with_description(Some(description.to_string()))
        .with_model(Some(model.to_string()))
        .with_temperature(Some(temperature));

    // Add extracted acts
    let mut act_map = std::collections::HashMap::new();
    let mut act_order = Vec::new();
    for (act_name, act_prompt) in acts {
        act_order.push(act_name.clone());
        act_map.insert(
            act_name.clone(),
            PartialAct::new(act_prompt, None, None, vec![], None),
        );
    }
    partial.with_acts(act_map);
    partial.with_act_order(act_order);

    // Convert to TOML
    let toml = partial.to_toml().map_err(|e| {
        warn!(error = ?e, "Failed to generate TOML from partial narrative");
        rmcp::ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            Cow::Owned(format!("Failed to generate TOML: {}", e)),
            None,
        )
    })?;

    // Validate the generated TOML
    match Validator::validate_toml(&toml) {
        validation if validation.is_valid() => {
            debug!("Generated valid narrative TOML");
            Ok(toml)
        }
        validation => {
            warn!(
                errors = validation.errors().len(),
                warnings = validation.warnings().len(),
                "Generated TOML has validation issues, attempting auto-fix"
            );
            // Try auto-fixing if validation failed
            let (fixed, _fixes) = auto_fix_common_issues(&toml);
            Ok(fixed)
        }
    }
}

/// Extract acts from a natural language description.
///
/// Uses simple heuristics to identify act boundaries:
/// - "then" keywords indicate sequential acts
/// - Commas may separate acts
/// - Verbs at sentence start often indicate acts
#[instrument(skip(description), fields(description_len = description.len()))]
fn extract_acts_from_description(description: &str) -> Vec<(String, String)> {
    use tracing::{debug, trace};

    let lower = description.to_lowercase();

    // Strategy 1: Split on "then" keyword
    if lower.contains("then") {
        let parts: Vec<&str> = description.split("then").map(|s| s.trim()).collect();
        debug!(
            strategy = "then_split",
            parts = parts.len(),
            "Splitting description on 'then'"
        );

        let acts: Vec<(String, String)> = parts
            .iter()
            .enumerate()
            .filter(|(_, part)| !part.is_empty())
            .map(|(i, part)| {
                // Extract first verb as act name
                let act_name =
                    extract_verb_from_phrase(part).unwrap_or_else(|| format!("act{}", i + 1));
                trace!(act_name = %act_name, prompt = %part, "Extracted act from 'then' clause");
                (act_name, part.to_string())
            })
            .collect();

        if !acts.is_empty() {
            return acts;
        }
    }

    // Strategy 2: Split on commas (for lists like "fetch, analyze, report")
    if lower.matches(',').count() >= 2 {
        let parts: Vec<&str> = description.split(',').map(|s| s.trim()).collect();
        debug!(
            strategy = "comma_split",
            parts = parts.len(),
            "Splitting description on commas"
        );

        let acts: Vec<(String, String)> = parts
            .iter()
            .enumerate()
            .filter(|(_, part)| !part.is_empty())
            .map(|(i, part)| {
                let act_name =
                    extract_verb_from_phrase(part).unwrap_or_else(|| format!("act{}", i + 1));
                trace!(act_name = %act_name, prompt = %part, "Extracted act from comma clause");
                (act_name, part.to_string())
            })
            .collect();

        if acts.len() >= 2 {
            return acts;
        }
    }

    // Strategy 3: Single act (default)
    debug!(strategy = "single_act", "Creating single-act narrative");
    vec![("main".to_string(), description.to_string())]
}

/// Extract a verb from the beginning of a phrase to use as an act name.
#[instrument(skip(phrase), fields(phrase_len = phrase.len()))]
fn extract_verb_from_phrase(phrase: &str) -> Option<String> {
    use tracing::trace;

    let words: Vec<&str> = phrase
        .trim_start_matches(|c: char| !c.is_alphabetic())
        .split_whitespace()
        .take(3)
        .collect();

    if words.is_empty() {
        return None;
    }

    // Common verbs that might start action descriptions
    let common_verbs = [
        "fetch",
        "get",
        "retrieve",
        "load",
        "read",
        "query",
        "download",
        "analyze",
        "process",
        "transform",
        "compute",
        "calculate",
        "examine",
        "generate",
        "create",
        "produce",
        "build",
        "construct",
        "write",
        "send",
        "post",
        "publish",
        "upload",
        "transmit",
        "deliver",
        "summarize",
        "report",
        "display",
        "show",
        "present",
        "output",
        "validate",
        "check",
        "verify",
        "test",
        "ensure",
    ];

    // Check if first word is a known verb
    let first = words[0].to_lowercase();
    if common_verbs.contains(&first.as_str()) {
        trace!(verb = %first, "Found common verb at start");
        return Some(first);
    }

    // Check if any word in first few is a verb
    for word in &words {
        let lower = word.to_lowercase();
        if common_verbs.contains(&lower.as_str()) {
            trace!(verb = %lower, "Found common verb in phrase");
            return Some(lower);
        }
    }

    trace!("No recognizable verb found");
    None
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
            Cow::Borrowed("Could not understand modification"),
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
            let updated = line
                .replace(&format!("\"{}\", ", act_name), "")
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
    use tracing::{debug, trace};

    let model = extract_model_name(modification)?;
    debug!(model = %model, "Extracted model name");

    let lines: Vec<&str> = toml.lines().collect();
    let mut result = Vec::new();
    let mut model_found = false;
    let mut in_narrative_section = false;
    let mut narrative_section_end = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Track if we're in [narrative] section
        if trimmed == "[narrative]" {
            in_narrative_section = true;
            trace!(line_num = i, "Found [narrative] section");
        } else if trimmed.starts_with('[') && in_narrative_section {
            // End of [narrative] section
            narrative_section_end = Some(i);
            in_narrative_section = false;
            trace!(line_num = i, "End of [narrative] section");
        }

        // If we find existing model line, replace it
        if trimmed.starts_with("model = ") {
            result.push(format!(r#"model = "{}""#, model));
            model_found = true;
            debug!(line_num = i, "Replaced existing model line");
        } else {
            result.push(line.to_string());
        }
    }

    // If model wasn't found, add it to [narrative] section
    if !model_found {
        if let Some(end_idx) = narrative_section_end {
            // Insert before the next section
            result.insert(end_idx, format!(r#"model = "{}""#, model));
            debug!(insert_at = end_idx, "Inserted model before next section");
        } else {
            // [narrative] section is last or only section, append at end
            result.push(format!(r#"model = "{}""#, model));
            debug!("Appended model to end of file");
        }
    }

    let modified = result.join("\n");
    let change = format!("Changed model to '{}'", model);

    Ok((modified, change))
}

/// Change the temperature in the narrative.
#[instrument(skip(toml), fields(toml_len = toml.len()))]
fn change_temperature(toml: &str, modification: &str) -> Result<(String, String), rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;
    use tracing::{debug, trace};

    let temperature = extract_temperature(modification)?;
    debug!(temperature = temperature, "Extracted temperature value");

    let lines: Vec<&str> = toml.lines().collect();
    let mut result = Vec::new();
    let mut changed = false;
    let mut in_narrative_section = false;
    let mut narrative_section_end = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Track if we're in [narrative] section
        if trimmed == "[narrative]" {
            in_narrative_section = true;
            trace!(line_num = i, "Found [narrative] section");
        } else if trimmed.starts_with('[') && in_narrative_section {
            // End of [narrative] section
            narrative_section_end = Some(i);
            in_narrative_section = false;
            trace!(line_num = i, "End of [narrative] section");
        }

        // If we find existing temperature line, replace it
        if trimmed.starts_with("temperature = ") {
            result.push(format!("temperature = {}", temperature));
            changed = true;
            debug!(line_num = i, "Replaced existing temperature line");
        } else {
            result.push(line.to_string());
        }
    }

    // If temperature wasn't found, add it to [narrative] section
    if !changed {
        if let Some(end_idx) = narrative_section_end {
            // Insert before the next section
            result.insert(end_idx, format!("temperature = {}", temperature));
            debug!(
                insert_at = end_idx,
                "Inserted temperature before next section"
            );
            changed = true;
        } else {
            // [narrative] section is last or only section, append at end
            result.push(format!("temperature = {}", temperature));
            debug!("Appended temperature to end of file");
            changed = true;
        }
    }

    if !changed {
        return Err(rmcp::ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            Cow::Borrowed("Could not add temperature field to TOML"),
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
    use tracing::debug;

    // Skip common instruction words to find the actual action verb
    let skip_words = [
        "add", "remove", "create", "delete", "act", "that", "which", "the", "a", "an",
    ];

    let words: Vec<&str> = text.split_whitespace().collect();

    // Find first significant verb after skipping common words
    for word in &words {
        let clean = word
            .trim_matches(|c: char| !c.is_alphabetic())
            .to_lowercase();
        if skip_words.contains(&clean.as_str()) {
            continue;
        }

        // Check if it's a recognized action verb
        if let Some(verb) = extract_verb_from_phrase(&clean) {
            debug!(verb = %verb, "Found action verb in modification");
            return verb;
        }

        // First non-skip word might be our verb
        if clean.len() > 2 {
            debug!(verb = %clean, "Using first significant word as verb");
            return clean;
        }
    }

    // Fallback: look for quoted names
    if let Some(start) = text.find('"') {
        if let Some(end) = text[start + 1..].find('"') {
            let name = &text[start + 1..start + 1 + end];
            debug!(name = %name, "Extracted quoted act name");
            return name.to_lowercase().replace(' ', "_");
        }
    }

    debug!("Using default act name");
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
