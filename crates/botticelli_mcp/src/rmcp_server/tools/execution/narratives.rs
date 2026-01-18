//! Narrative execution tools.
//!
//! Execute complete narratives from TOML files with prompt injection and parameter overrides.

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use crate::rmcp_server::helpers::{default_model, to_mcp_error};
#[cfg(not(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
)))]
use crate::rmcp_server::helpers::to_mcp_error;
use crate::rmcp_server::BotticelliServer;
use crate::{ExecuteNarrativeParams, ExecuteNarrativeResult};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::ErrorCode;
use std::borrow::Cow;
use tracing::{debug, instrument, trace, warn};

impl BotticelliServer {
    /// Execute a complete narrative from a TOML file.
    #[instrument(skip(self, params), fields(narrative_path = params.narrative_path(), model = ?params.model(), max_tokens = params.max_tokens()))]
    pub async fn execute_narrative(
        &self,
        Parameters(params): Parameters<ExecuteNarrativeParams>,
    ) -> Result<Json<ExecuteNarrativeResult>, rmcp::ErrorData> {
        let narrative_path = params.narrative_path().clone();
        let model = params.model().clone();
        let prompt = params.prompt();
        let max_tokens = *params.max_tokens();

        debug!(%narrative_path, model = ?model, prompt_len = prompt.len(), max_tokens, "Executing narrative");

        // Validate path exists
        let path = std::path::Path::new(&narrative_path);
        if !path.exists() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative file not found: {}", narrative_path)),
                None,
            ));
        }

        // Parse TOML and load narrative
        let (_narrative_name, mut narrative) = Self::parse_narrative_toml(path)?;

        // Apply runtime overrides (prompt injection + max_tokens)
        Self::apply_runtime_overrides(&mut narrative, prompt, max_tokens);

        #[cfg(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        ))]
        {
            use rmcp::model::ErrorCode;
            use std::borrow::Cow;
            use tracing::{debug, error};

            // Determine which driver to use based on model
            let model_str = model.unwrap_or_else(default_model);
            debug!(model = %model_str, "Selecting driver for narrative execution");
            
            // Use modified narrative for execution
            let narrative_source = botticelli_narrative::NarrativeSource::Single(Box::new(narrative));

            // Use helper from execution/helpers.rs to get trait object
            let driver = self.select_driver(&model_str)?;
            debug!(provider = driver.provider_name(), model = driver.model_name(), "Driver selected");
            
            let executor: botticelli_narrative::NarrativeExecutor<botticelli_error::NarrativeError> = 
                botticelli_narrative::NarrativeExecutor::new(driver);
            debug!("Executing narrative from source");
            
            let execution = executor
                .execute_from_source(&narrative_source)
                .await
                .map_err(|e| {
                    error!(error = %e, "Narrative execution failed");
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Owned(format!("Narrative execution failed: {}", e)),
                        None,
                    )
                })?;
            
            debug!(acts_count = execution.act_executions().len(), "Narrative execution completed");
            let result = Self::convert_execution_result(execution);
            return Ok(Json(result));
        }

        #[cfg(not(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        )))]
        {
            Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("No LLM drivers enabled. Enable at least one feature: gemini, anthropic, ollama, huggingface, or groq"),
                None,
            ))
        }
    }

    /// Parse narrative TOML file and return narrative name and loaded narrative.
    ///
    /// This handles:
    /// - Path validation
    /// - TOML file reading
    /// - Narrative name extraction ([narrative] or first from [narratives])
    /// - MultiNarrative loading
    /// - Narrative retrieval
    #[instrument(skip(path), fields(path = %path.display()))]
    fn parse_narrative_toml(
        path: &std::path::Path,
    ) -> Result<(String, botticelli_narrative::Narrative), rmcp::ErrorData> {
        debug!("Reading and parsing narrative TOML");

        // Read file content
        let content = std::fs::read_to_string(path).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Failed to read narrative file: {}", e)),
                None,
            )
        })?;

        // Parse to extract narrative names
        let toml_file: toml::Value = toml::from_str(&content).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Failed to parse TOML: {}", e)),
                None,
            )
        })?;

        // Extract narrative name (either from [narrative] or first from [narratives])
        let narrative_name = if let Some(narrative) = toml_file.get("narrative") {
            // Single narrative - get its name
            narrative
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        Cow::Borrowed("Narrative missing 'name' field"),
                        None,
                    )
                })?
                .to_string()
        } else if let Some(narratives) = toml_file.get("narratives") {
            // Multiple narratives - use first one
            narratives
                .as_table()
                .and_then(|table| table.keys().next())
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        Cow::Borrowed("No narratives found in TOML file"),
                        None,
                    )
                })?
                .clone()
        } else {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("No [narrative] or [narratives] section found in TOML"),
                None,
            ));
        };

        debug!(%narrative_name, "Loading narrative from TOML");

        // Now load the MultiNarrative with the identified name
        let multi = botticelli_narrative::MultiNarrative::from_file(path, &narrative_name)
            .map_err(|e| to_mcp_error(e, "Failed to load narrative"))?;

        // Get the active narrative
        let narrative = multi
            .get_narrative(&narrative_name)
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!(
                        "Narrative '{}' not found after loading",
                        narrative_name
                    )),
                    None,
                )
            })?
            .clone();

        debug!(%narrative_name, acts = narrative.acts().len(), "Narrative loaded successfully");

        Ok((narrative_name, narrative))
    }

    /// Apply runtime overrides to narrative (prompt injection + max_tokens).
    ///
    /// Modifies the narrative in-place:
    /// - Injects user prompt into first act's inputs
    /// - Applies max_tokens override to acts without explicit values
    #[instrument(skip(narrative, prompt), fields(prompt_len = prompt.len(), max_tokens))]
    fn apply_runtime_overrides(
        narrative: &mut botticelli_narrative::Narrative,
        prompt: &str,
        max_tokens: u32,
    ) {
        debug!("Applying runtime overrides to narrative");

        // Phase 1: Inject prompt into first act if provided and non-empty
        if !prompt.is_empty() {
            let first_act_name = narrative.toc().order().first().cloned();

            if let Some(act_name) = first_act_name {
                if let Some(act_config) = narrative.acts_mut().get_mut(&act_name) {
                    debug!(
                        act = %act_name,
                        prompt_len = prompt.len(),
                        original_input_count = act_config.inputs().len(),
                        "Injecting user prompt into first act"
                    );

                    // Prepend prompt as Input::Text to existing inputs
                    let mut new_inputs =
                        vec![botticelli_core::Input::Text(prompt.to_string())];
                    new_inputs.extend_from_slice(act_config.inputs());
                    let new_input_count = new_inputs.len();
                    act_config.set_inputs(new_inputs);

                    trace!(
                        act = %act_name,
                        new_input_count = new_input_count,
                        "Prompt injection complete"
                    );
                } else {
                    warn!(
                        act = %act_name,
                        "First act not found in acts map, cannot inject prompt"
                    );
                }
            } else {
                warn!("Narrative has no acts in TOC, cannot inject prompt");
            }
        } else {
            trace!("Prompt is empty, skipping injection");
        }

        // Phase 2: Apply max_tokens override to acts without explicit max_tokens
        let mut override_count = 0;
        for (act_name, act_config) in narrative.acts_mut().iter_mut() {
            if act_config.max_tokens().is_none() {
                debug!(
                    act = %act_name,
                    max_tokens = max_tokens,
                    "Applying max_tokens override"
                );
                act_config.set_max_tokens(Some(max_tokens));
                override_count += 1;
            } else {
                trace!(
                    act = %act_name,
                    act_max_tokens = ?act_config.max_tokens(),
                    "Act has explicit max_tokens, not overriding"
                );
            }
        }

        if override_count > 0 {
            debug!(
                acts_overridden = override_count,
                total_acts = narrative.acts_mut().len(),
                "Applied max_tokens override to {} acts",
                override_count
            );
        }
    }

    /// Convert NarrativeExecution to ExecuteNarrativeResult.
    ///
    /// Extracts:
    /// - Final output from last act
    /// - Models used from all acts
    /// - Total token usage
    #[cfg(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    ))]
    #[instrument(skip(execution), fields(acts = execution.act_executions().len()))]
    fn convert_execution_result(
        execution: botticelli_core::NarrativeExecution,
    ) -> ExecuteNarrativeResult {
        debug!("Converting execution result");

        // Extract final output from last act
        let final_output = execution
            .act_executions()
            .last()
            .map(|act| act.response().clone())
            .unwrap_or_else(|| "No acts executed".to_string());

        // Extract models used from acts
        let models_used: Vec<String> = execution
            .act_executions()
            .iter()
            .filter_map(|act| act.model().clone())
            .collect();

        // Calculate total tokens from execution
        let total_tokens = execution
            .total_token_usage()
            .map(|usage| *usage.total_tokens() as u32);

        debug!(
            final_output_len = final_output.len(),
            models_count = models_used.len(),
            ?total_tokens,
            "Conversion complete"
        );

        ExecuteNarrativeResult::new(
            final_output,
            execution.act_executions().len(),
            models_used,
            total_tokens,
            true,  // success - we got here without errors
            None,  // no error
        )
    }
}
