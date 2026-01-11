//! Utility functions for narrative execution.

use crate::StateManager;
use botticelli_core::{ActExecution, Output};
use botticelli_error::{BotticelliResult, NarrativeError, NarrativeErrorKind};
use serde_json::Value as JsonValue;
use tracing::instrument;

/// Capture and save ID fields from bot command output to state.
///
/// Extracts common ID fields (channel_id, message_id, role_id, etc.) from JSON response
/// and saves them to persistent state for later reference.
#[instrument(skip(state_mgr, result), fields(platform, command))]
pub(super) fn capture_bot_command_ids(
    state_mgr: &StateManager,
    platform: &str,
    command: &str,
    result: &JsonValue,
) -> BotticelliResult<()> {
    // Debug: Log the actual JSON response structure
    tracing::debug!(
        platform = %platform,
        command = %command,
        result = %serde_json::to_string_pretty(result).unwrap_or_else(|_| "invalid json".to_string()),
        "Bot command result for ID extraction"
    );

    // Load global state
    let mut state = state_mgr.load(&crate::state::StateScope::Global)?;

    // List of common ID field names to capture
    let id_fields = [
        "id", // Generic ID field (most common in Discord responses)
        "channel_id",
        "message_id",
        "role_id",
        "user_id",
        "guild_id",
        "emoji_id",
        "webhook_id",
        "integration_id",
        "invite_code",
        "thread_id",
        "event_id",
        "sticker_id",
    ];

    // Extract and save any ID fields found in the response
    let mut captured_count = 0;
    for id_field in &id_fields {
        if let Some(value) = result.get(id_field) {
            // Convert value to string
            let id_str = match value {
                JsonValue::String(s) => s.clone(),
                JsonValue::Number(n) => n.to_string(),
                _ => continue, // Skip non-scalar values
            };

            // Generate state key: <platform>.<command>.<field>
            let state_key = format!("{}.{}.{}", platform, command, id_field);

            // Also save with just the field name for convenient short-form access
            let short_key = id_field.to_string();

            state.set(&state_key, &id_str);
            state.set(&short_key, &id_str);

            tracing::debug!(
                key = %state_key,
                short_key = %short_key,
                value = %id_str,
                "Captured bot command ID to state"
            );

            captured_count += 1;
        }
    }

    // Save state back to disk
    if captured_count > 0 {
        state_mgr.save(&crate::state::StateScope::Global, &state)?;

        tracing::info!(
            platform = %platform,
            command = %command,
            count = captured_count,
            "Saved bot command IDs to state"
        );
    }

    Ok(())
}

/// Extract text content from LLM outputs.
///
/// Concatenates all text outputs with newlines between them.
#[instrument(skip(outputs), fields(output_count = outputs.len()))]
pub(super) fn extract_text_from_outputs(outputs: &[Output]) -> BotticelliResult<String> {
    tracing::debug!(output_count = outputs.len(), "Extracting text from outputs");

    let mut texts = Vec::new();

    for output in outputs {
        if let Output::Text(text) = output {
            texts.push(text.clone());
        }
    }

    let result = if texts.is_empty() {
        tracing::debug!("No text outputs found");
        String::new()
    } else {
        let joined = texts.join("\n");
        tracing::debug!(
            text_count = texts.len(),
            total_length = joined.len(),
            "Text outputs concatenated"
        );
        joined
    };

    Ok(result)
}

/// Resolve a state reference (${state:key}) from persistent storage.
#[instrument(skip(state_manager), fields(state_key))]
fn resolve_state_reference(
    state_key: &str,
    state_manager: Option<&StateManager>,
) -> BotticelliResult<String> {
    tracing::debug!(state_key = %state_key, "Resolving state reference");

    let state_mgr = state_manager.ok_or_else(|| {
        tracing::error!(state_key = %state_key, "State manager not configured");
        NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
            "State reference 'state:{}' requires state_manager to be configured",
            state_key
        )))
    })?;

    let state = state_mgr
        .load(&crate::state::StateScope::Global)
        .map_err(|e| {
            tracing::error!(state_key = %state_key, error = %e, "Failed to load state");
            NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
                "Failed to load state: {}",
                e
            )))
        })?;

    let value = state
        .get(state_key)
        .ok_or_else(|| {
            let available_keys: Vec<_> = state.keys().collect();
            tracing::warn!(
                state_key = %state_key,
                available_keys = ?available_keys,
                "State key not found"
            );
            NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
                "State key '{}' not found. Available keys: {}",
                state_key,
                if available_keys.is_empty() {
                    "none".to_string()
                } else {
                    available_keys.join(", ")
                }
            )))
        })?
        .to_string();

    tracing::debug!(state_key = %state_key, value_len = value.len(), "State reference resolved");
    Ok(value)
}

/// Resolve an environment variable reference (${env:VAR}).
#[instrument(fields(env_var))]
fn resolve_env_reference(env_var: &str) -> BotticelliResult<String> {
    tracing::debug!(env_var = %env_var, "Resolving environment variable");

    std::env::var(env_var)
        .map(|value| {
            tracing::debug!(env_var = %env_var, value_len = value.len(), "Environment variable resolved");
            value
        })
        .map_err(|e| {
            tracing::error!(env_var = %env_var, error = %e, "Environment variable not found");
            NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
                "Environment variable '{}' not found: {}",
                env_var, e
            )))
            .into()
        })
}

/// Resolve a reference to the previous act output ({{previous}}).
#[instrument(skip(act_executions), fields(current_index, act_count = act_executions.len()))]
fn resolve_previous_act(
    act_executions: &[ActExecution],
    current_index: usize,
) -> BotticelliResult<String> {
    tracing::debug!(current_index, "Resolving previous act reference");

    if current_index == 0 {
        tracing::error!("Cannot reference {{{{previous}}}} in first act");
        return Err(NarrativeError::new(NarrativeErrorKind::TemplateError(
            "Cannot reference {{previous}} in first act".to_string(),
        ))
        .into());
    }

    let response = act_executions[current_index - 1].response().clone();
    tracing::debug!(
        current_index,
        previous_index = current_index - 1,
        response_len = response.len(),
        "Previous act reference resolved"
    );
    Ok(response)
}

/// Navigate a JSON path within an act's response ({{act_name.field.path}}).
#[instrument(skip(act_executions), fields(act_name, json_path, act_count = act_executions.len()))]
fn resolve_json_path(
    act_name: &str,
    json_path: &str,
    act_executions: &[ActExecution],
) -> BotticelliResult<String> {
    tracing::debug!(
        act_name = %act_name,
        json_path = %json_path,
        "Resolving JSON path in act response"
    );

    let act_exec = act_executions
        .iter()
        .find(|exec| exec.act_name() == act_name)
        .ok_or_else(|| {
            tracing::error!(
                act_name = %act_name,
                available_acts = ?act_executions.iter().map(|e| e.act_name()).collect::<Vec<_>>(),
                "Referenced act not found"
            );
            NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
                "Referenced act '{}' not found in execution history",
                act_name
            )))
        })?;

    let json_value: JsonValue = serde_json::from_str(act_exec.response()).map_err(|e| {
        tracing::error!(
            act_name = %act_name,
            error = %e,
            "Act response is not valid JSON"
        );
        NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
            "Act '{}' response is not valid JSON: {}",
            act_name, e
        )))
    })?;

    let mut current = &json_value;
    for segment in json_path.split('.') {
        current = current.get(segment).ok_or_else(|| {
            tracing::error!(
                act_name = %act_name,
                json_path = %json_path,
                segment = %segment,
                "JSON path segment not found"
            );
            NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
                "JSON path '{}' not found in act '{}'",
                json_path, act_name
            )))
        })?;
    }

    let result = match current {
        JsonValue::String(s) => Ok(s.clone()),
        JsonValue::Number(n) => Ok(n.to_string()),
        JsonValue::Bool(b) => Ok(b.to_string()),
        JsonValue::Null => Ok("null".to_string()),
        _ => serde_json::to_string(current).map_err(|e| {
            tracing::error!(error = %e, "Failed to serialize JSON value");
            NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
                "Failed to serialize JSON value: {}",
                e
            )))
            .into()
        }),
    };

    if let Ok(ref value) = result {
        tracing::debug!(
            act_name = %act_name,
            json_path = %json_path,
            value_len = value.len(),
            "JSON path resolved"
        );
    }

    result
}

/// Resolve a simple act name reference ({{act_name}}).
#[instrument(skip(act_executions), fields(act_name, act_count = act_executions.len()))]
fn resolve_act_reference(
    act_name: &str,
    act_executions: &[ActExecution],
) -> BotticelliResult<String> {
    tracing::debug!(act_name = %act_name, "Resolving act reference");

    act_executions
        .iter()
        .find(|exec| exec.act_name() == act_name)
        .map(|exec| {
            let response = exec.response().clone();
            tracing::debug!(
                act_name = %act_name,
                response_len = response.len(),
                "Act reference resolved"
            );
            response
        })
        .ok_or_else(|| {
            tracing::error!(
                act_name = %act_name,
                available_acts = ?act_executions.iter().map(|e| e.act_name()).collect::<Vec<_>>(),
                "Referenced act not found"
            );
            NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
                "Referenced act '{}' not found in execution history",
                act_name
            )))
            .into()
        })
}

/// Resolve a single template reference based on its type.
#[instrument(skip(act_executions, state_manager), fields(reference, reference_type = tracing::field::Empty))]
fn resolve_reference(
    reference: &str,
    act_executions: &[ActExecution],
    current_index: usize,
    state_manager: Option<&StateManager>,
) -> BotticelliResult<String> {
    if let Some(state_key) = reference.strip_prefix("state:") {
        tracing::Span::current().record("reference_type", "state");
        resolve_state_reference(state_key, state_manager)
    } else if let Some(env_var) = reference.strip_prefix("env:") {
        tracing::Span::current().record("reference_type", "env");
        resolve_env_reference(env_var)
    } else if reference == "previous" {
        tracing::Span::current().record("reference_type", "previous");
        resolve_previous_act(act_executions, current_index)
    } else if reference.contains('.') {
        tracing::Span::current().record("reference_type", "json_path");
        let parts: Vec<&str> = reference.splitn(2, '.').collect();
        resolve_json_path(parts[0], parts[1], act_executions)
    } else {
        tracing::Span::current().record("reference_type", "act");
        resolve_act_reference(reference, act_executions)
    }
}

/// Resolve template placeholders in a string using act execution history and state.
///
/// Supports:
/// - `{{previous}}` - Content from the immediately previous act
/// - `{{act_name}}` - Content from a specific named act
/// - `{{act_name.field.path}}` - JSON path within an act's response
/// - `${state:key}` - Value from persistent state
/// - `${env:VAR}` - Environment variable value
///
/// # Errors
///
/// Returns error if:
/// - Referenced act doesn't exist
/// - Referenced act hasn't executed yet
/// - Template syntax is malformed
/// - State key doesn't exist
/// - Environment variable doesn't exist
/// - JSON path is invalid
#[instrument(skip(template, act_executions, state_manager), fields(template_len = template.len(), act_count = act_executions.len(), current_index, has_state = state_manager.is_some()))]
pub(super) fn resolve_template(
    template: &str,
    act_executions: &[ActExecution],
    current_index: usize,
    state_manager: Option<&StateManager>,
) -> BotticelliResult<String> {
    let mut result = template.to_string();

    // Find all {{...}} or ${...} patterns
    let re = regex::Regex::new(r"(?:\{\{([^}]+)\}\}|\$\{([^}]+)\})").map_err(|e| {
        NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
            "Invalid template regex: {}",
            e
        )))
    })?;

    for cap in re.captures_iter(template) {
        let placeholder = &cap[0];
        let reference = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().trim())
            .ok_or_else(|| {
                NarrativeError::new(NarrativeErrorKind::TemplateError(format!(
                    "Failed to extract reference from placeholder: {}",
                    placeholder
                )))
            })?;

        let replacement =
            resolve_reference(reference, act_executions, current_index, state_manager)?;
        result = result.replace(placeholder, &replacement);
    }

    Ok(result)
}
