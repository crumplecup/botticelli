use crate::tools::elicitation::PartialNarrativeRegistry;
use botticelli_error::{McpError, McpResult};
use botticelli_narrative::CarouselConfig;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitCarouselInput {
    pub narrative_id: String,
    pub level: CarouselLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_name: Option<String>,
    pub iterations: u32,
    #[serde(default)]
    pub continue_on_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_tokens_per_iteration: Option<u32>,
    #[serde(default = "default_budget_multiplier")]
    pub budget_multiplier: f64,
}

fn default_budget_multiplier() -> f64 {
    2.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CarouselLevel {
    Narrative,
    Act,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitCarouselOutput {
    pub success: bool,
    pub carousel_config: CarouselSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarouselSummary {
    pub level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_name: Option<String>,
    pub iterations: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_total_tokens: Option<u32>,
    pub budget_warnings: Vec<String>,
}

#[instrument(skip(registry), fields(narrative_id, level, iterations))]
pub async fn elicit_carousel(
    registry: &PartialNarrativeRegistry,
    input: ElicitCarouselInput,
) -> McpResult<ElicitCarouselOutput> {
    debug!("Eliciting carousel configuration");

    let narrative_id = Uuid::parse_str(&input.narrative_id)
        .map_err(|e| McpError::invalid_input(format!("Invalid narrative_id: {}", e)))?;

    if input.iterations == 0 || input.iterations > 1000 {
        return Err(McpError::invalid_input(
            "Iterations must be between 1 and 1000".to_string(),
        ));
    }

    match input.level {
        CarouselLevel::Act => {
            if input.act_name.is_none() {
                return Err(McpError::invalid_input(
                    "act_name required when level=act".to_string(),
                ));
            }
        }
        CarouselLevel::Narrative => {
            if input.act_name.is_some() {
                return Err(McpError::invalid_input(
                    "act_name should not be provided when level=narrative".to_string(),
                ));
            }
        }
    }

    registry
        .update_narrative(narrative_id, |partial| {
            let estimated_tokens = input.estimated_tokens_per_iteration.unwrap_or(1000) as u64;
            let carousel = CarouselConfig::new(input.iterations, estimated_tokens)
                .with_continue_on_error(input.continue_on_error);

            match input.level {
                CarouselLevel::Narrative => {
                    partial.carousel = Some(carousel);
                    debug!("Set narrative-level carousel");
                }
                CarouselLevel::Act => {
                    let act_name = input.act_name.as_ref().unwrap();
                    let act = partial
                        .acts
                        .get_mut(act_name)
                        .ok_or_else(|| McpError::invalid_input(format!("Act '{}' not found", act_name)))?;
                    act.carousel = Some(carousel);
                    debug!(act = %act_name, "Set act-level carousel");
                }
            }

            Ok(())
        })
        .await?;

    let estimated_total_tokens = input
        .estimated_tokens_per_iteration
        .map(|tokens| tokens * input.iterations);

    let mut budget_warnings = Vec::new();
    if let Some(total) = estimated_total_tokens {
        if total > 100_000 {
            budget_warnings.push(format!(
                "Estimated {} tokens may exceed default budget limits",
                total
            ));
        }
    }

    let level_str = match input.level {
        CarouselLevel::Narrative => "narrative",
        CarouselLevel::Act => "act",
    };

    Ok(ElicitCarouselOutput {
        success: true,
        carousel_config: CarouselSummary {
            level: level_str.to_string(),
            act_name: input.act_name,
            iterations: input.iterations,
            estimated_total_tokens,
            budget_warnings,
        },
    })
}
