use crate::tools::McpTool;
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use botticelli_interface::ElicitationRegistryOperations;
use botticelli_narrative::CarouselConfig;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

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

/// MCP tool for creating carousel configurations during narrative elicitation.
pub struct ElicitCarouselTool<R>
where
    R: ElicitationRegistryOperations<crate::PartialNarrative>,
{
    registry: Arc<R>,
}

impl<R> ElicitCarouselTool<R>
where
    R: ElicitationRegistryOperations<
            crate::PartialNarrative,
            Error = botticelli_error::BotticelliError,
        >,
{
    /// Creates a new carousel elicitation tool.
    pub fn new(registry: Arc<R>) -> Self {
        Self { registry }
    }

    #[tracing::instrument(skip(self))]
    async fn handle_carousel(&self, input: ElicitCarouselInput) -> McpResult<ElicitCarouselOutput> {
        tracing::debug!("Creating carousel configuration");

        // Verify narrative exists
        let _ = self
            .registry
            .get_narrative(&input.narrative_id)
            .map_err(|e| McpError::execution_failed(e.to_string()))?;

        // Create carousel config
        let carousel_config = CarouselConfig::new(
            input.iterations,
            input.estimated_tokens_per_iteration.unwrap_or(1000) as u64,
        )
        .with_continue_on_error(input.continue_on_error);

        // Calculate budget warnings if estimate provided
        let mut budget_warnings = Vec::new();
        if let Some(tokens_per_iter) = input.estimated_tokens_per_iteration {
            let total_estimated = tokens_per_iter * input.iterations;
            let budget_threshold = (total_estimated as f64 * input.budget_multiplier) as u32;

            if total_estimated > 10_000 {
                budget_warnings.push(format!(
                    "High token estimate: {} tokens across {} iterations",
                    total_estimated, input.iterations
                ));
            }

            if budget_threshold > 50_000 {
                budget_warnings.push(format!(
                    "Budget threshold very high: {} tokens ({}x multiplier)",
                    budget_threshold, input.budget_multiplier
                ));
            }
        }

        // Update narrative with carousel config
        self.registry
            .update_narrative(&input.narrative_id, |partial| -> Result<(), _> {
                match input.level {
                    CarouselLevel::Narrative => {
                        partial.with_carousel(Some(carousel_config.clone()));
                        Ok(())
                    }
                    CarouselLevel::Act => {
                        let act_name = match input.act_name.as_ref() {
                            Some(name) => name,
                            None => {
                                return Err(botticelli_error::BotticelliError::from(
                                    McpError::invalid_input(
                                        "act_name required for Act level carousel",
                                    ),
                                ));
                            }
                        };

                        match partial.acts().get(act_name) {
                            Some(act) => {
                                let mut updated_act = act.clone();
                                updated_act.set_carousel(Some(carousel_config.clone()));
                                partial.acts_mut().insert(act_name.clone(), updated_act);
                                Ok(())
                            }
                            None => Err(botticelli_error::BotticelliError::from(
                                McpError::invalid_input(format!("Act '{}' not found", act_name)),
                            )),
                        }
                    }
                }
            })
            .map_err(|e| McpError::execution_failed(e.to_string()))?;

        tracing::debug!(
            narrative_id = %input.narrative_id,
            level = ?input.level,
            iterations = input.iterations,
            "Carousel configuration created"
        );

        Ok(ElicitCarouselOutput {
            success: true,
            carousel_config: CarouselSummary {
                level: match input.level {
                    CarouselLevel::Narrative => "narrative".to_string(),
                    CarouselLevel::Act => "act".to_string(),
                },
                act_name: input.act_name,
                iterations: input.iterations,
                estimated_total_tokens: input
                    .estimated_tokens_per_iteration
                    .map(|t| t * input.iterations),
                budget_warnings,
            },
        })
    }
}

#[async_trait]
impl<
    R: ElicitationRegistryOperations<
            crate::PartialNarrative,
            Error = botticelli_error::BotticelliError,
        > + Send
        + Sync,
> McpTool for ElicitCarouselTool<R>
{
    fn name(&self) -> &str {
        "elicit_carousel"
    }

    fn description(&self) -> &str {
        "Create a carousel configuration for iterative narrative refinement at narrative or act level"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "narrative_id": {
                    "type": "string",
                    "description": "UUID of the narrative being created"
                },
                "level": {
                    "type": "string",
                    "enum": ["narrative", "act"],
                    "description": "Level at which to apply carousel (narrative-wide or specific act)"
                },
                "act_name": {
                    "type": "string",
                    "description": "Name of the act (required if level is 'act')"
                },
                "iterations": {
                    "type": "integer",
                    "description": "Number of carousel iterations",
                    "minimum": 1
                },
                "continue_on_error": {
                    "type": "boolean",
                    "description": "Whether to continue if an iteration fails",
                    "default": false
                },
                "estimated_tokens_per_iteration": {
                    "type": "integer",
                    "description": "Estimated token usage per iteration for budget tracking"
                },
                "budget_multiplier": {
                    "type": "number",
                    "description": "Safety multiplier for budget calculations",
                    "default": 2.0,
                    "minimum": 1.0
                }
            },
            "required": ["narrative_id", "level", "iterations"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let input: ElicitCarouselInput = serde_json::from_value(input)
            .map_err(|e| McpError::invalid_input(format!("Invalid input: {}", e)))?;

        let output = self.handle_carousel(input).await?;

        serde_json::to_value(output)
            .map_err(|e| McpError::execution_failed(format!("Failed to serialize output: {}", e)))
    }
}
