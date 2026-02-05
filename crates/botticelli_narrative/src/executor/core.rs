//! Core executor implementation.

use crate::{
    ActConfig, CarouselConfig, CarouselResult, CarouselState, MultiNarrative, NarrativeMetadata,
    ProcessorRegistry, StateManager,
};
use botticelli_core::{ActExecutionBuilder, NarrativeExecution};
use botticelli_core::{GenerateRequest, Input, Message, MessageBuilder, Role};
use botticelli_error::{
    BackendError, BotticelliError, BotticelliResult, NarrativeError, NarrativeErrorKind,
};
use botticelli_interface::{BotCommandRegistry, NarrativeProvider, TableQueryRegistry};
use rmcp::tool;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tracing::instrument;

/// Executes narratives by calling LLM APIs in sequence.
///
/// The executor processes each act in the narrative's table of contents order,
/// passing previous act outputs as context to subsequent acts.
///
/// Optionally, processors can be registered to extract and process structured
/// data from act responses (e.g., JSON extraction, database insertion).
///
/// Bot commands can be registered to enable narratives to query social media
/// platforms (Discord, Slack, etc.) for real-time data.
///
/// Table queries can be registered to enable narratives to reference data
/// from database tables in prompts.
///
/// ## Template Substitution
///
/// Bot command arguments support template substitution using `{{act_name}}` or
/// `{{act_name.field.path}}` syntax to reference outputs from previous acts.
/// The response from each act is stored in the ActExecution history and can be
/// referenced by name. For JSON responses (e.g., from bot commands), you can
/// navigate JSON paths using dot notation.
#[derive(derive_getters::Getters)]
pub struct NarrativeExecutor<BE = botticelli_error::NarrativeError>
where
    BE: std::error::Error + Send + Sync + 'static,
{
    pub(super) driver: std::sync::Arc<
        dyn botticelli_interface::ExecutionDriver<
                GenerateRequest,
                botticelli_core::GenerateResponse,
            >,
    >,
    pub(super) processor_registry: Option<ProcessorRegistry>,
    pub(super) bot_registry: Option<Box<dyn BotCommandRegistry<Error = BE>>>,
    pub(super) table_registry:
        Option<Box<dyn TableQueryRegistry<Error = botticelli_error::DatabaseError>>>,
    pub(super) state_manager: Option<StateManager>,
}

impl<BE> NarrativeExecutor<BE>
where
    BE: std::error::Error + Send + Sync + 'static,
{
    /// Create a new narrative executor with the given LLM driver.
    ///
    /// This is a convenience method equivalent to using the builder:
    ///
    /// ```rust,ignore
    /// NarrativeExecutorBuilder::default()
    ///     .driver(driver)
    ///     .build()
    ///     .expect("Valid executor")
    /// ```
    #[instrument(skip(driver))]
    #[tool]
    pub fn new(
        driver: std::sync::Arc<
            dyn botticelli_interface::ExecutionDriver<
                    GenerateRequest,
                    botticelli_core::GenerateResponse,
                >,
        >,
    ) -> Self {
        tracing::debug!("Creating narrative executor");
        Self {
            driver,
            processor_registry: None,
            bot_registry: None,
            table_registry: None,
            state_manager: None,
        }
    }

    /// Create a new narrative executor with processors.
    ///
    /// Convenience method for creating an executor with a processor registry.
    /// Equivalent to:
    ///
    /// ```rust,ignore
    /// NarrativeExecutorBuilder::default()
    ///     .driver(driver)
    ///     .processor_registry(Some(registry))
    ///     .build()
    ///     .expect("Valid executor")
    /// ```
    #[instrument(skip(driver, registry), fields(processor_count = registry.len()))]
    #[tool]
    pub fn with_processors(
        driver: std::sync::Arc<
            dyn botticelli_interface::ExecutionDriver<
                    GenerateRequest,
                    botticelli_core::GenerateResponse,
                >,
        >,
        registry: ProcessorRegistry,
    ) -> Self {
        tracing::debug!(
            processor_count = registry.len(),
            "Creating executor with processors"
        );
        Self {
            driver,
            processor_registry: Some(registry),
            bot_registry: None,
            table_registry: None,
            state_manager: None,
        }
    }

    /// Execute a narrative, processing all acts in sequence.
    ///
    /// Each act sees the outputs from all previous acts as conversation history.
    /// The first act receives just its prompt, the second act sees the first act's
    /// response plus its own prompt, and so on.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any LLM API call fails
    /// - The response format is unexpected
    #[instrument(skip(self, narrative))]
    #[tool]
    pub fn execute<'a, N>(
        &'a self,
        narrative: &'a N,
    ) -> Pin<Box<dyn Future<Output = BotticelliResult<NarrativeExecution>> + Send + 'a>>
    where
        N: NarrativeProvider<
                Metadata = NarrativeMetadata,
                ActConfig = ActConfig,
                CarouselConfig = CarouselConfig,
            > + ?Sized,
    {
        Box::pin(async move { self.execute_impl(narrative).await })
    }

    /// Execute a narrative from a NarrativeSource.
    ///
    /// This is the preferred method for executing narratives as it automatically
    /// handles composition context. The NarrativeSource enum encapsulates whether
    /// the narrative needs MultiNarrative context for composition.
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails.
    #[instrument(skip(self, source))]
    pub async fn execute_from_source(
        &self,
        source: &crate::NarrativeSource,
    ) -> BotticelliResult<NarrativeExecution> {
        match source {
            crate::NarrativeSource::Single(narrative) => {
                // No composition context needed
                self.execute_impl(narrative.as_ref()).await
            }
            crate::NarrativeSource::MultiWithContext {
                multi,
                execute_name,
            } => {
                // Get the narrative to execute
                let narrative = multi.get_narrative(execute_name).ok_or_else(|| {
                    NarrativeError::new(NarrativeErrorKind::TomlParse(format!(
                        "Narrative '{}' not found in MultiNarrative",
                        execute_name
                    )))
                })?;

                // Execute with full MultiNarrative context for composition
                self.execute_impl_with_multi(narrative, Some(multi)).await
            }
        }
    }

    /// Execute a narrative by loading it from a TOML file and selecting a specific narrative by name.
    ///
    /// This is a convenience method for bots that need to execute narratives dynamically.
    /// It loads a multi-narrative file and executes the specified narrative within it.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be loaded
    /// - The narrative name is not found
    /// - Execution fails
    #[instrument(skip(self), fields(path, narrative_name))]
    pub async fn execute_narrative_by_name(
        &self,
        path: &str,
        narrative_name: &str,
    ) -> BotticelliResult<NarrativeExecution> {
        use crate::MultiNarrative;
        use std::path::Path;

        let multi = MultiNarrative::from_file(Path::new(path), narrative_name)?;

        let narrative = multi.get_narrative(narrative_name).ok_or_else(|| {
            NarrativeError::new(NarrativeErrorKind::TomlParse(format!(
                "Narrative '{}' not found in {}",
                narrative_name, path
            )))
        })?;

        // Execute with the MultiNarrative for composition support
        self.execute_impl_with_multi(narrative, Some(&multi)).await
    }

    #[tracing::instrument(
        skip(self, narrative),
        fields(
            narrative_name = narrative.name(),
            act_count = narrative.act_names().len(),
            has_processors = self.processor_registry.is_some(),
            has_bot_registry = self.bot_registry.is_some(),
            has_table_registry = self.table_registry.is_some(),
            has_state_manager = self.state_manager.is_some(),
        )
    )]
    async fn execute_impl<N>(&self, narrative: &N) -> BotticelliResult<NarrativeExecution>
    where
        N: NarrativeProvider<
                Metadata = NarrativeMetadata,
                ActConfig = ActConfig,
                CarouselConfig = CarouselConfig,
            > + ?Sized,
    {
        tracing::info!("Starting narrative execution");
        self.execute_impl_with_multi(narrative, None).await
    }

    #[tracing::instrument(
        skip(self, narrative, multi),
        fields(
            narrative_name = narrative.name(),
            act_count = narrative.act_names().len(),
            has_multi_context = multi.is_some(),
            has_processors = self.processor_registry.is_some(),
            has_bot_registry = self.bot_registry.is_some(),
            has_table_registry = self.table_registry.is_some(),
            has_state_manager = self.state_manager.is_some(),
            total_duration_ms = tracing::field::Empty,
        )
    )]
    pub(super) async fn execute_impl_with_multi<N>(
        &self,
        narrative: &N,
        multi: Option<&MultiNarrative>,
    ) -> BotticelliResult<NarrativeExecution>
    where
        N: NarrativeProvider<
                Metadata = NarrativeMetadata,
                ActConfig = ActConfig,
                CarouselConfig = CarouselConfig,
            > + ?Sized,
    {
        let start_time = Instant::now();
        tracing::info!("Starting narrative execution with multi-narrative context");
        let mut act_executions = Vec::new();
        let mut conversation_history: Vec<Message> = Vec::new();

        for (sequence_number, act_name) in narrative.act_names().iter().enumerate() {
            let span = tracing::info_span!(
                "execute_act",
                act = %act_name,
                sequence = sequence_number,
                total_acts = narrative.act_names().len(),
            );
            let _enter = span.enter();
            // Get the configuration for this act
            let config = narrative.get_act_config(act_name).ok_or_else(|| {
                NarrativeError::new(NarrativeErrorKind::MissingAct(act_name.to_string()))
            })?;

            // Check if this act is a narrative reference
            if config.is_narrative_ref() {
                let (act_execution, message) = self
                    .execute_narrative_composition(act_name, &config, multi, sequence_number)
                    .await?;

                act_executions.push(act_execution);
                conversation_history.push(message);
                continue;
            }

            // Process inputs (execute bot commands, query tables, etc.)
            // Pass execution history for template resolution
            let (processed_inputs, bot_command_result) = self
                .process_inputs(narrative, config.inputs(), &act_executions, sequence_number)
                .await?;

            // Check if this is an action-only act (no text inputs from TOML that need LLM processing)
            // Bot command results in processed_inputs should NOT trigger LLM calls
            let has_text_prompt = config
                .inputs()
                .iter()
                .any(|input| matches!(input, Input::Text(text) if !text.trim().is_empty()));

            let (response_text, model, temperature, max_tokens, token_usage, duration) =
                if has_text_prompt {
                    // This act needs an LLM response
                    self.execute_llm_act::<N>(
                        act_name,
                        narrative,
                        &config,
                        processed_inputs.clone(),
                        &mut conversation_history,
                    )
                    .await?
                } else {
                    // Action-only act - no LLM call needed
                    tracing::debug!(
                        act = %act_name,
                        "Skipping LLM call for action-only act"
                    );
                    // Use bot command result as response if available, otherwise generic success message
                    let response_text = if let Some(result) = bot_command_result {
                        serde_json::to_string(&result)
                            .unwrap_or_else(|_| "Action completed successfully".to_string())
                    } else {
                        "Action completed successfully".to_string()
                    };
                    (response_text, None, None, None, None, None)
                };

            // Create the act execution (store processed inputs)
            let act_execution = ActExecutionBuilder::default()
                .act_name(act_name.clone())
                .inputs(processed_inputs.clone())
                .model(model)
                .temperature(temperature)
                .max_tokens(max_tokens)
                .response(response_text.clone())
                .sequence_number(sequence_number)
                .token_usage(token_usage)
                .estimated_cost_usd(None) // TODO: Calculate from token_usage + model pricing
                .duration_ms(duration.map(|d| d.as_millis() as u64))
                .build()
                .map_err(|e| {
                    BotticelliError::from(BackendError::new(format!(
                        "Failed to build ActExecution: {}",
                        e
                    )))
                })?;

            tracing::debug!(
                act = %act_name,
                act_execution_response_length = act_execution.response().len(),
                "ActExecution created with response"
            );

            // Process with registered processors
            self.process_with_processors::<N>(narrative, &config, &act_execution, sequence_number)
                .await?;

            // Store the act execution
            act_executions.push(act_execution);

            // Add the assistant's response to conversation history for the next act (only if there was an LLM call)
            if has_text_prompt {
                conversation_history.push(
                    MessageBuilder::default()
                        .role(Role::Assistant)
                        .content(vec![Input::Text(response_text)])
                        .build()
                        .map_err(|e| {
                            NarrativeError::new(NarrativeErrorKind::ConfigurationError(format!(
                                "Failed to build message: {}",
                                e
                            )))
                        })?,
                );

                // Apply history retention policies
                Self::apply_history_retention(act_name, &mut conversation_history)?;
            }
        }

        let total_duration = start_time.elapsed();

        // Record metrics in span
        tracing::Span::current().record("total_duration_ms", total_duration.as_millis() as u64);

        tracing::info!(
            total_acts = act_executions.len(),
            total_duration_ms = total_duration.as_millis(),
            avg_duration_per_act_ms = if !act_executions.is_empty() {
                total_duration.as_millis() / act_executions.len() as u128
            } else {
                0
            },
            "Narrative execution completed"
        );

        // Calculate totals
        let (total_token_usage, total_duration_ms) = Self::calculate_total_metrics(&act_executions);

        Ok(NarrativeExecution::new(
            narrative.name().to_string(),
            act_executions,
            total_token_usage,
            None, // TODO: Calculate from total_token_usage + model pricing
            total_duration_ms,
        ))
    }

    /// respecting rate limit budgets and stopping when limits are approached.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Carousel configuration is missing
    /// - Budget cannot be created from rate limits
    /// - Any iteration fails (if continue_on_error is false)
    #[tracing::instrument(skip(self, narrative), fields(narrative_name = narrative.name()))]
    pub async fn execute_carousel<N>(&self, narrative: &N) -> BotticelliResult<CarouselResult>
    where
        N: NarrativeProvider<
                Metadata = NarrativeMetadata,
                ActConfig = ActConfig,
                CarouselConfig = CarouselConfig,
            > + ?Sized,
    {
        // Get carousel config from narrative
        let carousel_config = narrative.carousel_config().ok_or_else(|| {
            botticelli_error::NarrativeError::new(
                botticelli_error::NarrativeErrorKind::ConfigurationError(
                    "Narrative does not have carousel configuration".to_string(),
                ),
            )
        })?;

        tracing::info!(
            iterations = carousel_config.iterations(),
            estimated_tokens = carousel_config.estimated_tokens_per_iteration(),
            continue_on_error = carousel_config.continue_on_error(),
            "Starting carousel execution"
        );

        // Create carousel state with budget
        tracing::debug!("Getting rate limits for carousel");
        let rate_limits = self.driver.rate_limits();
        tracing::debug!("Rate limits retrieved for carousel");
        let mut state = CarouselState::new(carousel_config.clone(), rate_limits);

        let mut executions = Vec::new();

        while state.can_continue() {
            if let Err(e) = state.start_iteration() {
                tracing::error!(error = %e, "Failed to start carousel iteration");
                break;
            }

            match self.execute(narrative).await {
                Ok(execution) => {
                    tracing::debug!(
                        iteration = state.current_iteration(),
                        acts = execution.act_executions().len(),
                        "Iteration completed successfully"
                    );

                    // Consume tokens from budget
                    // TODO: Track actual token usage from driver response
                    let estimated_tokens = *carousel_config.estimated_tokens_per_iteration();
                    if let Err(e) = state.budget_mut().consume(estimated_tokens) {
                        tracing::warn!(
                            error = %e,
                            "Failed to consume tokens from budget"
                        );
                    }

                    state.record_success();
                    executions.push(execution);
                }
                Err(e) => {
                    tracing::error!(
                        iteration = state.current_iteration(),
                        error = %e,
                        "Iteration failed"
                    );

                    state.record_failure();

                    if !carousel_config.continue_on_error() {
                        tracing::warn!("Stopping carousel due to error (continue_on_error=false)");
                        break;
                    }
                }
            }
        }

        state.finish();
        let result = CarouselResult::from(&state);

        tracing::info!(
            iterations = result.iterations_attempted(),
            successful = result.successful_iterations(),
            failed = result.failed_iterations(),
            completed = result.completed(),
            budget_exhausted = result.budget_exhausted(),
            "Carousel execution finished"
        );

        Ok(result)
    }
}
