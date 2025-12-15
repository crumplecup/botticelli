use crate::tools::elicitation::PartialNarrativeRegistry;
use botticelli_error::{McpError, McpResult};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNarrativeStateInput {
    pub narrative_id: String,
    #[serde(default = "default_format")]
    pub format: StateFormat,
}

fn default_format() -> StateFormat {
    StateFormat::Summary
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StateFormat {
    Summary,
    Full,
    Toml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNarrativeStateOutput {
    pub narrative_id: String,
    pub state: NarrativeStateSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toml: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeStateSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub acts_count: usize,
    pub acts: Vec<String>,
    pub completeness: String,
    pub has_carousel: bool,
}

#[instrument(skip(registry), fields(narrative_id, format))]
pub async fn get_narrative_state(
    registry: &PartialNarrativeRegistry,
    input: GetNarrativeStateInput,
) -> McpResult<GetNarrativeStateOutput> {
    debug!("Getting narrative state");

    let narrative_id = Uuid::parse_str(&input.narrative_id)
        .map_err(|e| McpError::invalid_input(format!("Invalid narrative_id: {}", e)))?;

    let partial = registry.get_narrative(&narrative_id.to_string())?;

    let acts_count = partial.acts.len();
    let acts: Vec<String> = partial.act_order.clone();
    let has_carousel =
        partial.carousel.is_some() || partial.acts.values().any(|act| act.carousel.is_some());

    let metadata_complete =
        partial.name.is_some() && partial.description.is_some() && partial.model.is_some();
    let acts_complete =
        !partial.acts.is_empty() && partial.acts.values().all(|act| !act.prompt.is_empty());
    let inputs_partial = partial.acts.values().any(|act| !act.inputs.is_empty());

    let mut completeness_score = 0;
    if metadata_complete {
        completeness_score += 33;
    }
    if acts_complete {
        completeness_score += 33;
    }
    if inputs_partial {
        completeness_score += 34;
    }

    let toml = if matches!(input.format, StateFormat::Toml) {
        match partial.to_toml() {
            Ok(toml_str) => Some(toml_str),
            Err(e) => {
                return Err(McpError::invalid_input(format!(
                    "Failed to convert to TOML: {}",
                    e
                )))
            }
        }
    } else {
        None
    };

    Ok(GetNarrativeStateOutput {
        narrative_id: input.narrative_id,
        state: NarrativeStateSummary {
            name: partial.name.clone(),
            acts_count,
            acts,
            completeness: format!("{}%", completeness_score),
            has_carousel,
        },
        toml,
    })
}
