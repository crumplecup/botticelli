use crate::tools::elicitation::PartialNarrativeRegistry;
use crate::tools::McpTool;
use async_trait::async_trait;
use botticelli_error::{McpError, McpResult};
use botticelli_narrative::CarouselConfig;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, instrument};

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
pub struct ElicitCarouselTool {
    registry: PartialNarrativeRegistry,
}

impl ElicitCarouselTool {
    /// Creates a new carousel elicitation tool.
    pub fn new(registry: PartialNarrativeRegistry) -> Self {
        Self { registry }
    }

    #[instrument(skip(self))]
    async fn handle_carousel(&self, input: ElicitCarouselInput) -> McpResult<ElicitCarouselOutput> {
        // Verify narrative exists
        let _ = self.registry.get(&input.narrative_id)?;

        // Create carousel config
        let carousel_config = CarouselConfig::new(
            input.iterations,
            input.estimated_tokens_per_iteration.unwrap_or(1000) as u64
        ).with_continue_on_error(input.continue_on_error);

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
        let mut update = serde_json::json!({});
        
        match input.level {
            CarouselLevel::Narrative => {
                update["carousel"] = serde_json::to_value(&carousel_config)
                    .map_err(|e| McpError::execution_failed(format!("Failed to serialize carousel config: {}", e)))?;
            }
            CarouselLevel::Act => {
                let act_name = input.act_name.as_ref().ok_or_else(|| {
                    McpError::invalid_input("act_name required for Act level carousel")
                })?;
                
                // Update the specific act's carousel
                update["acts"] = serde_json::json!({
                    act_name: {
                        "carousel": carousel_config
                    }
                });
            }
        }
        
        self.registry.update(&input.narrative_id, update)?;

        debug!(
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
impl McpTool for ElicitCarouselTool {
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
