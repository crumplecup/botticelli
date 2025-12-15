use crate::tools::elicitation::PartialNarrativeRegistry;
use botticelli_error::{McpError, McpResult};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNarrativeFieldInput {
    pub narrative_id: String,
    pub updates: Vec<FieldUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldUpdate {
    pub path: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNarrativeFieldOutput {
    pub success: bool,
    pub updated_fields: Vec<String>,
    pub validation: ValidationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[instrument(skip(registry), fields(narrative_id, update_count = input.updates.len()))]
pub async fn update_narrative_field(
    registry: &PartialNarrativeRegistry,
    input: UpdateNarrativeFieldInput,
) -> McpResult<UpdateNarrativeFieldOutput> {
    debug!("Updating narrative fields");

    let narrative_id = Uuid::parse_str(&input.narrative_id)
        .map_err(|e| McpError::invalid_input(format!("Invalid narrative_id: {}", e)))?;

    let updates = input.updates;
    let mut updated_fields = Vec::new();
    let mut errors = Vec::new();

    registry
        .update_narrative(&narrative_id.to_string(), |partial| {
            for update in &updates {
                match apply_field_update(partial, &update.path, &update.value) {
                    Ok(()) => {
                        updated_fields.push(update.path.clone());
                        debug!(field = %update.path, "Updated field");
                    }
                    Err(e) => {
                        errors.push(format!("Failed to update '{}': {}", update.path, e));
                    }
                }
            }
            Ok(())
        })?;

    Ok(UpdateNarrativeFieldOutput {
        success: errors.is_empty(),
        updated_fields,
        validation: ValidationSummary {
            errors,
            warnings: Vec::new(),
        },
    })
}

fn apply_field_update(
    partial: &mut crate::PartialNarrative,
    path: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    let parts: Vec<&str> = path.split('.').collect();

    match parts.as_slice() {
        ["name"] => {
            partial.name = Some(value.as_str().ok_or("name must be a string")?.to_string());
        }
        ["description"] => {
            partial.description = Some(
                value
                    .as_str()
                    .ok_or("description must be a string")?
                    .to_string(),
            );
        }
        ["model"] => {
            partial.model = Some(value.as_str().ok_or("model must be a string")?.to_string());
        }
        ["temperature"] => {
            partial.temperature = Some(value.as_f64().ok_or("temperature must be a number")?);
        }
        ["max_tokens"] => {
            partial.max_tokens = Some(value.as_u64().ok_or("max_tokens must be a number")? as u32);
        }
        ["acts", act_name, "prompt"] => {
            let act = partial
                .acts
                .get_mut(*act_name)
                .ok_or_else(|| format!("Act '{}' not found", act_name))?;
            act.prompt = value.as_str().ok_or("prompt must be a string")?.to_string();
        }
        ["acts", act_name, "model"] => {
            let act = partial
                .acts
                .get_mut(*act_name)
                .ok_or_else(|| format!("Act '{}' not found", act_name))?;
            act.model = Some(value.as_str().ok_or("model must be a string")?.to_string());
        }
        ["acts", act_name, "temperature"] => {
            let act = partial
                .acts
                .get_mut(*act_name)
                .ok_or_else(|| format!("Act '{}' not found", act_name))?;
            act.temperature = Some(value.as_f64().ok_or("temperature must be a number")?);
        }
        _ => {
            return Err(format!("Unsupported field path: {}", path));
        }
    }

    Ok(())
}
