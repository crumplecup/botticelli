//! Elicitation tools for interactive narrative creation.
//!
//! Tools that prompt users for input during narrative creation process.

use crate::rmcp_server::helpers::to_mcp_error;
use crate::rmcp_server::BotticelliServer;
use crate::{
    CarouselLevel, CarouselSummary, ElicitActParams, ElicitActResult, ElicitBoolParams,
    ElicitBoolResult, ElicitCarouselParams, ElicitCarouselResult, ElicitMetadataParams,
    ElicitMetadataResult, ElicitNumberParams, ElicitNumberResult, ElicitSelectParams,
    ElicitSelectResult, ElicitTextParams, ElicitTextResult,
};
use rmcp::handler::server::wrapper::{Json, Parameters};
use tracing::{debug, instrument};

impl BotticelliServer {
    /// Prompt user for text input.
    #[instrument(skip(self, params), fields(prompt_len = params.prompt().len()))]
    pub async fn elicit_text(
        &self,
        Parameters(params): Parameters<ElicitTextParams>,
    ) -> Result<Json<ElicitTextResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        let prompt = params.prompt().clone();
        debug!(?prompt, "Eliciting text input");

        // Check if dialog resource is available
        let dialog = self.dialog.as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Dialog resource not configured"),
                None,
            )
        })?;

        // Ask for text input
        let text = dialog
            .ask_text(&prompt)
            .await
            .map_err(|e| to_mcp_error(e, "Dialog error"))?;

        debug!(response_len = text.len(), "Received text input");

        let result = ElicitTextResult::new(text);
        Ok(Json(result))
    }
    
    /// Prompt user for yes/no input.
    #[instrument(skip(self, params), fields(prompt_len = params.prompt().len(), default = params.default()))]
    pub async fn elicit_bool(
        &self,
        Parameters(params): Parameters<ElicitBoolParams>,
    ) -> Result<Json<ElicitBoolResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        let prompt = params.prompt().clone();
        let default = params.default().clone();
        debug!(?prompt, default, "Eliciting boolean confirmation");

        // Check if dialog resource is available
        let dialog = self.dialog.as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Dialog resource not configured"),
                None,
            )
        })?;

        // Ask for confirmation
        let confirmed = dialog
            .ask_confirmation(&prompt, default)
            .await
            .map_err(|e| to_mcp_error(e, "Dialog error"))?;

        debug!(confirmed, "Received boolean confirmation");

        let result = ElicitBoolResult::new(confirmed);
        Ok(Json(result))
    }
    
    /// Prompt user for numeric input within a range.
    #[instrument(skip(self, params), fields(prompt_len = params.prompt().len(), min = params.min(), max = params.max()))]
    pub async fn elicit_number(
        &self,
        Parameters(params): Parameters<ElicitNumberParams>,
    ) -> Result<Json<ElicitNumberResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        let prompt = params.prompt().clone();
        let min = *params.min();
        let max = *params.max();
        debug!(?prompt, min, max, "Eliciting numeric input");

        // Validate range
        if min > max {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Invalid range: min ({}) > max ({})", min, max)),
                None,
            ));
        }

        // Check if dialog resource is available
        let dialog = self.dialog.as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Dialog resource not configured"),
                None,
            )
        })?;

        // Ask for number input
        let number = dialog
            .ask_number(&prompt, min, max)
            .await
            .map_err(|e| to_mcp_error(e, "Dialog error"))?;

        debug!(number, "Received numeric input");

        let result = ElicitNumberResult::new(number);
        Ok(Json(result))
    }
    
    /// Prompt user to select from a list of options.
    #[instrument(skip(self, params), fields(prompt_len = params.prompt().len(), option_count = params.options().len()))]
    pub async fn elicit_select(
        &self,
        Parameters(params): Parameters<ElicitSelectParams>,
    ) -> Result<Json<ElicitSelectResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        let prompt = params.prompt().clone();
        let options = params.options().clone();
        debug!(?prompt, option_count = options.len(), "Eliciting selection");

        // Validate options is not empty
        if options.is_empty() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("Options array cannot be empty"),
                None,
            ));
        }

        // Check if dialog resource is available
        let dialog = self.dialog.as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Dialog resource not configured"),
                None,
            )
        })?;

        // Convert to &str array for dialog API
        let option_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();

        // Ask for selection
        let index = dialog
            .ask_choice(&prompt, &option_refs)
            .await
            .map_err(|e| to_mcp_error(e, "Dialog error"))?;

        // Get selected option
        let selected = options.get(index).ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Invalid index {} (max {})", index, options.len())),
                None,
            )
        })?;

        debug!(selected = %selected, index, "Received selection");

        let result = ElicitSelectResult::new(selected.clone());
        Ok(Json(result))
    }
    
    /// Update narrative metadata (name, description, defaults).
    #[instrument(skip(self, params), fields(narrative_id = params.narrative_id(), name = ?params.name(), description = ?params.description()))]
    pub async fn elicit_metadata(
        &self,
        Parameters(params): Parameters<ElicitMetadataParams>,
    ) -> Result<Json<ElicitMetadataResult>, rmcp::ErrorData> {
        let narrative_id = params.narrative_id().clone();
        let name = params.name().clone();
        let description = params.description().clone();
        let default_model = params.default_model().clone();
        let default_temperature = params.default_temperature().clone();

        debug!(narrative_id, "Updating narrative metadata");

        // Update fields if provided
        self.narrative_registry
            .update(
                &narrative_id,
                serde_json::json!({
                    "name": name,
                    "description": description,
                    "model": default_model,
                    "temperature": default_temperature,
                }),
            )
            .map_err(|e| to_mcp_error(e, "Failed to update metadata"))?;

        debug!(narrative_id, "Metadata updated");

        Ok(Json(ElicitMetadataResult::new(
            narrative_id,
            "updated".to_string(),
        )))
    }
    
    /// Elicit and refine a narrative act's content.
    #[instrument(skip(self, params), fields(narrative_id = params.narrative_id(), act_name = params.act_name(), prompt_len = params.prompt().len()))]
    pub async fn elicit_act(
        &self,
        Parameters(params): Parameters<ElicitActParams>,
    ) -> Result<Json<ElicitActResult>, rmcp::ErrorData> {
        let narrative_id = params.narrative_id().clone();
        let act_name = params.act_name().clone();
        let prompt = params.prompt().clone();
        let model = params.model().clone();
        let temperature = params.temperature().clone();

        debug!(narrative_id, act_name, "Eliciting act");

        // Get current narrative
        let mut partial = self
            .narrative_registry
            .get(&narrative_id)
            .map_err(|e| to_mcp_error(e, "Narrative not found"))?;

        // Check if act exists
        let status = if partial.acts().contains_key(&act_name) {
            "updated"
        } else {
            partial.act_order_mut().push(act_name.clone());
            "created"
        };

        // Create or update act
        partial.acts_mut().insert(
            act_name.clone(),
            crate::PartialAct::new(prompt, model, temperature, vec![], None),
        );

        // Update registry
        self.narrative_registry.add(partial);

        debug!(narrative_id, act_name, status, "Act elicited");

        Ok(Json(ElicitActResult::new(
            narrative_id,
            act_name,
            status.to_string(),
        )))
    }
    
    /// Run iterative carousel refinement on narrative acts.
    #[instrument(skip(self, params), fields(narrative_id = params.narrative_id(), level = ?params.level(), iterations = params.iterations()))]
    pub async fn elicit_carousel(
        &self,
        Parameters(params): Parameters<ElicitCarouselParams>,
    ) -> Result<Json<ElicitCarouselResult>, rmcp::ErrorData> {
        use botticelli_narrative::CarouselConfig;
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        let narrative_id = params.narrative_id().clone();
        let level = params.level().clone();
        let act_name = params.act_name().clone();
        let iterations = *params.iterations();
        let continue_on_error = *params.continue_on_error();
        let estimated_tokens_per_iteration = params.estimated_tokens_per_iteration().clone();
        let budget_multiplier = *params.budget_multiplier();

        debug!(
            narrative_id,
            ?level,
            iterations,
            "Creating carousel configuration"
        );

        // Get narrative
        let mut partial = self.narrative_registry.get(&narrative_id).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative not found: {}", e)),
                None,
            )
        })?;

        // Create carousel config
        let carousel_config = CarouselConfig::new(
            iterations,
            estimated_tokens_per_iteration.unwrap_or(1000) as u64,
        )
        .with_continue_on_error(continue_on_error);

        // Calculate budget warnings
        let mut budget_warnings = Vec::new();
        if let Some(tokens_per_iter) = estimated_tokens_per_iteration {
            let total_estimated = tokens_per_iter * iterations;
            let budget_threshold = (total_estimated as f64 * budget_multiplier) as u32;

            if total_estimated > 10_000 {
                budget_warnings.push(format!(
                    "High token estimate: {} tokens across {} iterations",
                    total_estimated, iterations
                ));
            }

            if budget_threshold > 50_000 {
                budget_warnings.push(format!(
                    "Budget threshold very high: {} tokens ({}x multiplier)",
                    budget_threshold, budget_multiplier
                ));
            }
        }

        // Apply carousel based on level
        match level {
            CarouselLevel::Narrative => {
                partial.with_carousel(Some(carousel_config));
            }
            CarouselLevel::Act => {
                let act_name_ref = act_name.as_ref().ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        Cow::Borrowed("act_name required for Act level carousel"),
                        None,
                    )
                })?;

                match partial.acts().get(act_name_ref) {
                    Some(act) => {
                        let mut updated_act = act.clone();
                        updated_act.set_carousel(Some(carousel_config));
                        partial.acts_mut().insert(act_name_ref.clone(), updated_act);
                    }
                    None => {
                        return Err(rmcp::ErrorData::new(
                            ErrorCode::INVALID_PARAMS,
                            Cow::Owned(format!("Act '{}' not found", act_name_ref)),
                            None,
                        ));
                    }
                }
            }
        }

        // Update narrative in registry
        self.narrative_registry.add(partial);

        debug!(
            narrative_id,
            ?level,
            iterations,
            "Carousel configuration created"
        );

        let carousel_config = CarouselSummary::new(
            match level {
                CarouselLevel::Narrative => "narrative".to_string(),
                CarouselLevel::Act => "act".to_string(),
            },
            act_name,
            iterations,
            estimated_tokens_per_iteration.map(|t| t * iterations),
            budget_warnings,
        );

        Ok(Json(ElicitCarouselResult::new(true, carousel_config)))
    }
}
