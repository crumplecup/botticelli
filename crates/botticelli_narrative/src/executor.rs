//! Narrative execution logic.
//!
//! This module provides the executor that processes multi-act narratives
//! by calling LLM APIs in sequence, passing context between acts.

use crate::{
    CarouselResult, CarouselState, MultiNarrative, NarrativeProvider, ProcessorContext,
    ProcessorRegistry, StateManager,
};
use botticelli_core::{GenerateRequest, Input, Message, MessageBuilder, Output, Role};
use botticelli_error::{BotticelliError, BotticelliResult, NarrativeError, NarrativeErrorKind};
use botticelli_interface::{
    ActExecution, BotticelliDriver, NarrativeExecution, TableQueryRegistry,
};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Trait for executing bot commands (platform-agnostic).
///
/// This is defined here to avoid circular dependencies between
/// botticelli_narrative and botticelli_social. Implementations
/// live in botticelli_social.
#[async_trait::async_trait]
pub trait BotCommandRegistry: Send + Sync {
    /// Execute a bot command on a specific platform.
    async fn execute(
        &self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>>;
}

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
pub struct NarrativeExecutor<D: BotticelliDriver> {
    driver: D,
    processor_registry: Option<ProcessorRegistry>,
    bot_registry: Option<Box<dyn BotCommandRegistry>>,
    table_registry: Option<Box<dyn TableQueryRegistry>>,
    state_manager: Option<StateManager>,
}

impl<D: BotticelliDriver> NarrativeExecutor<D> {
    /// Create a new narrative executor with the given LLM driver.
    pub fn new(driver: D) -> Self {
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
    /// Processors will be invoked after each act completes to extract
    /// and process structured data from the response.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use botticelli_narrative::{NarrativeExecutor, ProcessorRegistry};
    ///
    /// let mut registry = ProcessorRegistry::new();
    /// registry.register(Box::new(MyProcessor::new()));
    ///
    /// let executor = NarrativeExecutor::with_processors(driver, registry);
    /// ```
    pub fn with_processors(driver: D, registry: ProcessorRegistry) -> Self {
        Self {
            driver,
            processor_registry: Some(registry),
            bot_registry: None,
            table_registry: None,
            state_manager: None,
        }
    }

    /// Add a bot command registry for executing platform commands.
    ///
    /// Enables narratives to execute bot commands and include results in prompts.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use botticelli_narrative::NarrativeExecutor;
    /// use botticelli_social::{BotCommandRegistry, DiscordCommandExecutor};
    ///
    /// let mut bot_registry = BotCommandRegistry::new();
    /// bot_registry.register(DiscordCommandExecutor::new("TOKEN"));
    ///
    /// let executor = NarrativeExecutor::new(driver)
    ///     .with_bot_registry(Box::new(bot_registry));
    /// ```
    pub fn with_bot_registry(mut self, registry: Box<dyn BotCommandRegistry>) -> Self {
        self.bot_registry = Some(registry);
        self
    }

    /// Add a table query registry for querying database tables.
    ///
    /// Enables narratives to reference data from database tables in prompts.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use botticelli_narrative::NarrativeExecutor;
    /// use botticelli_database::TableQueryExecutor;
    ///
    /// let table_registry = TableQueryExecutor::new(connection);
    ///
    /// let executor = NarrativeExecutor::new(driver)
    ///     .with_table_registry(Box::new(table_registry));
    /// ```
    pub fn with_table_registry(mut self, registry: Box<dyn TableQueryRegistry>) -> Self {
        self.table_registry = Some(registry);
        self
    }

    /// Add a state manager for persistent state across narrative executions.
    ///
    /// Enables narratives to store and retrieve state like channel IDs, message IDs,
    /// and other runtime artifacts that persist between runs.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use botticelli_narrative::{NarrativeExecutor, StateManager};
    ///
    /// let state_manager = StateManager::new("./state")?;
    ///
    /// let executor = NarrativeExecutor::new(driver)
    ///     .with_state_manager(state_manager);
    /// ```
    pub fn with_state_manager(mut self, manager: StateManager) -> Self {
        self.state_manager = Some(manager);
        self
    }

    /// Capture and save ID fields from bot command output to state.
    ///
    /// Extracts common ID fields (channel_id, message_id, role_id, etc.) from JSON response
    /// and saves them to persistent state for later reference.
    fn capture_bot_command_ids(
        &self,
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
        let mut state = state_mgr
            .load(&crate::state::StateScope::Global)
            .map_err(|e| {
                NarrativeError::new(NarrativeErrorKind::StateError(format!(
                    "Failed to load state: {}",
                    e
                )))
            })?;

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
            state_mgr
                .save(&crate::state::StateScope::Global, &state)
                .map_err(|e| {
                    NarrativeError::new(NarrativeErrorKind::StateError(format!(
                        "Failed to save state: {}",
                        e
                    )))
                })?;

            tracing::info!(
                platform = %platform,
                command = %command,
                count = captured_count,
                "Saved bot command IDs to state"
            );
        }

        Ok(())
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
    pub fn execute<'a, N>(
        &'a self,
        narrative: &'a N,
    ) -> Pin<Box<dyn Future<Output = BotticelliResult<NarrativeExecution>> + Send + 'a>>
    where
        N: NarrativeProvider + ?Sized,
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
    async fn execute_impl<N: NarrativeProvider + ?Sized>(
        &self,
        narrative: &N,
    ) -> BotticelliResult<NarrativeExecution> {
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
        )
    )]
    async fn execute_impl_with_multi<N: NarrativeProvider + ?Sized>(
        &self,
        narrative: &N,
        multi: Option<&MultiNarrative>,
    ) -> BotticelliResult<NarrativeExecution> {
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
            let config = narrative
                .get_act_config(act_name)
                .expect("NarrativeProvider should ensure all acts exist");

            // Check if this act is a narrative reference
            if config.is_narrative_ref() {
                let narrative_ref_name = config.narrative_ref().as_ref().unwrap();
                tracing::info!(
                    act = %act_name,
                    referenced_narrative = %narrative_ref_name,
                    "Executing narrative composition"
                );

                // Try to resolve the referenced narrative
                // First try from the MultiNarrative context if available
                if let Some(ref_narrative) = if let Some(m) = multi {
                    m.get_narrative(narrative_ref_name)
                } else {
                    None
                } {
                    // Recursively execute the referenced narrative with the same multi context
                    tracing::debug!("Recursively executing referenced narrative from multi");
                    let nested_execution =
                        Box::pin(self.execute_impl_with_multi(ref_narrative, multi)).await?;

                    // Collect all responses from the nested execution
                    let nested_responses: Vec<String> = nested_execution
                        .act_executions
                        .iter()
                        .map(|e| e.response.clone())
                        .collect();

                    let combined_response = nested_responses.join("\n\n");

                    tracing::info!(
                        act_count = nested_execution.act_executions.len(),
                        response_len = combined_response.len(),
                        "Completed nested narrative execution"
                    );

                    // Record the composition as a single act
                    act_executions.push(ActExecution {
                        act_name: act_name.clone(),
                        inputs: Vec::new(),
                        model: config.model().clone(),
                        temperature: *config.temperature(),
                        max_tokens: *config.max_tokens(),
                        response: combined_response.clone(),
                        sequence_number,
                    });

                    // Add the combined response to conversation history
                    conversation_history.push(
                        MessageBuilder::default()
                            .role(Role::Assistant)
                            .content(vec![Input::Text(combined_response)])
                            .build()
                            .map_err(|e| {
                                NarrativeError::new(NarrativeErrorKind::ConfigurationError(
                                    format!("Failed to build message: {}", e),
                                ))
                            })?,
                    );
                } else {
                    // Narrative not found - this is an error
                    return Err(NarrativeError::new(
                        NarrativeErrorKind::ConfigurationError(format!(
                            "Referenced narrative '{}' not found. Narrative composition requires MultiNarrative.",
                            narrative_ref_name
                        ))
                    ).into());
                }

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

            let (response_text, model, temperature, max_tokens) = if has_text_prompt {
                // This act needs an LLM response
                // Build the request with conversation history + processed inputs
                tracing::debug!(
                    processed_inputs_count = processed_inputs.len(),
                    "Building LLM request with processed inputs"
                );

                for (idx, input) in processed_inputs.iter().enumerate() {
                    match input {
                        Input::Text(text) => {
                            let preview = text
                                .char_indices()
                                .take(100)
                                .last()
                                .map(|(idx, _)| &text[..=idx])
                                .unwrap_or(text);
                            tracing::debug!(
                                input_index = idx,
                                text_length = text.len(),
                                text_preview = preview,
                                "Processed input is Text"
                            );
                        }
                        other => {
                            tracing::debug!(
                                input_index = idx,
                                input_type = ?other,
                                "Processed input is non-Text"
                            );
                        }
                    }
                }

                conversation_history.push(
                    MessageBuilder::default()
                        .role(Role::User)
                        .content(processed_inputs.clone())
                        .build()
                        .map_err(|e| {
                            NarrativeError::new(NarrativeErrorKind::ConfigurationError(format!(
                                "Failed to build message: {}",
                                e
                            )))
                        })?,
                );

                // Apply narrative-level defaults for model/temperature/max_tokens if act doesn't override
                let metadata = narrative.metadata();
                let model = config.model().clone().or_else(|| metadata.model().clone());
                let temperature = config.temperature().or_else(|| *metadata.temperature());
                let max_tokens = config.max_tokens().or_else(|| *metadata.max_tokens());

                let request = GenerateRequest::builder()
                    .messages(conversation_history.clone())
                    .max_tokens(max_tokens)
                    .temperature(temperature)
                    .model(model.clone())
                    .build()
                    .map_err(|e| {
                        BotticelliError::from(NarrativeError::new(NarrativeErrorKind::FileRead(
                            format!("Failed to build request: {}", e),
                        )))
                    })?;

                // Call the LLM
                let llm_span = tracing::info_span!(
                    "llm_call",
                    act = %act_name,
                    model = ?request.model(),
                    temperature = ?request.temperature(),
                    max_tokens = ?request.max_tokens(),
                    message_count = request.messages().len(),
                );

                let response = {
                    let _enter = llm_span.enter();
                    tracing::info!("Calling LLM API");
                    let result = self.driver.generate(&request).await?;
                    tracing::info!(
                        outputs_count = result.outputs().len(),
                        "LLM response received"
                    );
                    result
                };

                // Debug log the output types
                for (idx, output) in response.outputs().iter().enumerate() {
                    match output {
                        botticelli_core::Output::Text(text) => {
                            let preview: String = text.chars().take(100).collect();
                            let preview = preview.as_str();
                            tracing::debug!(
                                output_index = idx,
                                text_length = text.len(),
                                text_preview = preview,
                                "Output is Text variant"
                            );
                        }
                        other => {
                            tracing::debug!(
                                output_index = idx,
                                output_type = ?other,
                                "Output is non-Text variant"
                            );
                        }
                    }
                }

                // Extract text from response
                let response_text = extract_text_from_outputs(response.outputs())?;

                let preview = response_text.chars().take(200).collect::<String>();
                tracing::debug!(
                    response_length = response_text.len(),
                    response_preview = preview,
                    "Response text extracted from LLM outputs"
                );

                (response_text, model, temperature, max_tokens)
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
                (response_text, None, None, None)
            };

            // Create the act execution (store processed inputs)
            let act_execution = ActExecution {
                act_name: act_name.clone(),
                inputs: processed_inputs.clone(),
                model,
                temperature,
                max_tokens,
                response: response_text.clone(),
                sequence_number,
            };

            tracing::debug!(
                act = %act_name,
                act_execution_response_length = act_execution.response.len(),
                "ActExecution created with response"
            );

            // Process with registered processors
            if let Some(registry) = &self.processor_registry {
                let processor_span = tracing::info_span!(
                    "process_act",
                    act = %act_name,
                    processors = registry.len(),
                );
                let _enter = processor_span.enter();

                tracing::info!("Processing act with registered processors");

                // Determine if this is the last act in the narrative
                let is_last_act = sequence_number == narrative.act_names().len() - 1;

                // Determine if we should extract output:
                // - If extract_output is explicitly set, use that value
                // - Otherwise, only extract for the last act (default behavior)
                let should_extract_output = config.extract_output().unwrap_or(is_last_act);

                tracing::debug!(
                    is_last_act,
                    extract_config = ?config.extract_output(),
                    should_extract_output,
                    "Determined extraction policy"
                );

                // Build processor context
                let context = ProcessorContext {
                    execution: &act_execution,
                    narrative_metadata: narrative.metadata(),
                    narrative_name: narrative.name(),
                    is_last_act,
                    should_extract_output,
                };

                if let Err(e) = registry.process(&context).await {
                    tracing::error!(
                        error = %e,
                        "Act processing failed, continuing execution"
                    );
                    // Note: We don't fail the entire narrative on processor errors
                    // The user still gets the execution results
                }
            }

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

                // Apply history retention policies to the user message we just processed
                // The user message is at conversation_history.len() - 2 (assistant message was just pushed)
                if conversation_history.len() >= 2 {
                    let user_msg_idx = conversation_history.len() - 2;
                    if let Some(user_message) = conversation_history.get(user_msg_idx) {
                        // Apply retention policies to the message content
                        let updated_content = crate::history_retention::apply_retention_to_inputs(
                            user_message.content(),
                        );

                        // Only replace if content changed
                        if updated_content.len() != user_message.content().len()
                            || updated_content
                                .iter()
                                .zip(user_message.content().iter())
                                .any(|(a, b)| a != b)
                        {
                            // Create new message with updated content
                            let updated_message = MessageBuilder::default()
                                .role(*user_message.role())
                                .content(updated_content)
                                .build()
                                .map_err(|e| {
                                    NarrativeError::new(NarrativeErrorKind::ConfigurationError(
                                        format!(
                                            "Failed to build message with retention policy: {}",
                                            e
                                        ),
                                    ))
                                })?;

                            // Replace the old message
                            conversation_history[user_msg_idx] = updated_message;

                            tracing::debug!(
                                act = %act_name,
                                "Applied history retention policies to user message"
                            );
                        }
                    }
                }
            }
        }

        Ok(NarrativeExecution {
            narrative_name: narrative.name().to_string(),
            act_executions,
        })
    }

    /// Execute a narrative in a carousel loop with budget management.
    ///
    /// Runs the narrative multiple times according to the carousel configuration,
    /// respecting rate limit budgets and stopping when limits are approached.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Carousel configuration is missing
    /// - Budget cannot be created from rate limits
    /// - Any iteration fails (if continue_on_error is false)
    #[tracing::instrument(skip(self, narrative), fields(narrative_name = narrative.name()))]
    pub async fn execute_carousel<N: NarrativeProvider + ?Sized>(
        &self,
        narrative: &N,
    ) -> BotticelliResult<CarouselResult> {
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
        let mut state = CarouselState::new(carousel_config.clone(), *self.driver.rate_limits());

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
                        acts = execution.act_executions.len(),
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
        let result = CarouselResult::from_state(&state);

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

    /// Get a reference to the underlying LLM driver.
    pub fn driver(&self) -> &D {
        &self.driver
    }

    /// Process inputs, executing bot commands and converting them to text.
    ///
    /// For each input:
    /// - BotCommand: Execute via registry and format result as JSON text
    /// - Table: (Future) Query database and format result
    /// - Other: Pass through unchanged
    ///
    /// Bot command arguments can use template syntax to inject previous act outputs:
    /// - `{{previous}}` - Output from immediately previous act
    /// - `{{act_name}}` - Output from specific named act
    #[tracing::instrument(
        skip(self, narrative, inputs, act_executions),
        fields(
            current_index,
            input_count = inputs.len(),
            bot_commands = 0,
            tables = 0
        )
    )]
    #[tracing::instrument(
        skip(self, narrative, inputs, act_executions),
        fields(
            narrative_name = narrative.name(),
            input_count = inputs.len(),
            act_index = current_index,
        )
    )]
    async fn process_inputs<N: NarrativeProvider + ?Sized>(
        &self,
        narrative: &N,
        inputs: &[Input],
        act_executions: &[ActExecution],
        current_index: usize,
    ) -> BotticelliResult<(Vec<Input>, Option<JsonValue>)> {
        tracing::debug!("Processing inputs for act");
        let mut processed = Vec::new();
        let mut bot_command_count = 0;
        let mut table_count = 0;
        let mut last_bot_command_result: Option<JsonValue> = None;

        for input in inputs {
            match input {
                Input::BotCommand {
                    platform,
                    command,
                    args,
                    required,
                    ..
                } => {
                    bot_command_count += 1;
                    tracing::debug!(
                        platform = %platform,
                        command = %command,
                        required = %required,
                        "Processing bot command input"
                    );

                    let registry = self.bot_registry.as_ref().ok_or_else(|| {
                        let msg = format!(
                            "Bot command '{}' requires bot_registry to be configured",
                            command
                        );
                        tracing::error!(platform = %platform, command = %command, msg);
                        botticelli_error::NarrativeError::new(
                            botticelli_error::NarrativeErrorKind::BotCommandNotConfigured(msg),
                        )
                    })?;

                    // Resolve templates in bot command arguments
                    let mut resolved_args = args.clone();
                    for (_key, value) in resolved_args.iter_mut() {
                        if let JsonValue::String(s) = value {
                            *s = resolve_template(
                                s,
                                act_executions,
                                current_index,
                                self.state_manager.as_ref(),
                            )?;
                        }
                    }

                    match registry.execute(platform, command, &resolved_args).await {
                        Ok(result) => {
                            // Store bot command result for potential use as act output
                            last_bot_command_result = Some(result.clone());

                            // Extract and save IDs to state if state_manager is available
                            if let Some(state_mgr) = &self.state_manager {
                                self.capture_bot_command_ids(
                                    state_mgr, platform, command, &result,
                                )?;
                            }

                            // Convert JSON result to pretty-printed text for LLM context
                            let text = serde_json::to_string_pretty(&result).map_err(|e| {
                                tracing::error!(error = %e, "Failed to serialize bot command result");
                                botticelli_error::NarrativeError::new(
                                    botticelli_error::NarrativeErrorKind::SerializationError(
                                        format!("Bot command result serialization failed: {}", e),
                                    ),
                                )
                            })?;

                            tracing::info!(
                                platform = %platform,
                                command = %command,
                                result_length = text.len(),
                                "Bot command executed successfully"
                            );

                            processed.push(Input::Text(text));
                        }
                        Err(e) => {
                            if *required {
                                tracing::error!(
                                    platform = %platform,
                                    command = %command,
                                    error = %e,
                                    "Required bot command failed, halting execution"
                                );
                                return Err(botticelli_error::NarrativeError::new(
                                    botticelli_error::NarrativeErrorKind::BotCommandFailed(
                                        format!("Required command '{}' failed: {}", command, e),
                                    ),
                                )
                                .into());
                            } else {
                                tracing::warn!(
                                    platform = %platform,
                                    command = %command,
                                    error = %e,
                                    "Optional bot command failed, continuing with error message"
                                );
                                let error_msg =
                                    format!("[Bot command '{}' failed: {}]", command, e);
                                processed.push(Input::Text(error_msg));
                            }
                        }
                    }
                }

                Input::Table {
                    table_name,
                    columns,
                    where_clause,
                    limit,
                    offset,
                    order_by,
                    format,
                    ..
                } => {
                    table_count += 1;

                    let format_str = match format {
                        botticelli_core::TableFormat::Json => "json",
                        botticelli_core::TableFormat::Markdown => "markdown",
                        botticelli_core::TableFormat::Csv => "csv",
                    };

                    tracing::debug!(
                        table_name = %table_name,
                        format = %format_str,
                        "Processing table reference input"
                    );

                    let registry = self.table_registry.as_ref().ok_or_else(|| {
                        let msg = format!(
                            "Table reference '{}' requires table_registry to be configured",
                            table_name
                        );
                        tracing::error!(table_name = %table_name, msg);
                        botticelli_error::NarrativeError::new(
                            botticelli_error::NarrativeErrorKind::TableQueryNotConfigured(msg),
                        )
                    })?;

                    // Build query view
                    let mut query_builder = botticelli_interface::TableQueryViewBuilder::default();
                    query_builder.table_name(table_name.to_string());

                    if let Some(cols) = columns.as_ref() {
                        query_builder.columns(cols.clone());
                    }
                    if let Some(where_str) = where_clause.as_ref() {
                        query_builder.filter(where_str.clone());
                    }
                    if let Some(lim) = limit {
                        query_builder.limit(*lim as i64);
                    }
                    if let Some(off) = offset {
                        query_builder.offset(*off as i64);
                    }
                    if let Some(order) = order_by.as_ref() {
                        query_builder.order_by(order.clone());
                    }
                    query_builder.format(format_str.to_string());

                    let query_view = query_builder.build().map_err(|e| {
                        botticelli_error::NarrativeError::new(
                            botticelli_error::NarrativeErrorKind::TableQueryNotConfigured(format!(
                                "Failed to build query: {}",
                                e
                            )),
                        )
                    })?;

                    match registry.query_table(&query_view).await {
                        Ok(result) => {
                            tracing::info!(
                                table_name = %table_name,
                                result_length = result.len(),
                                "Table query executed successfully"
                            );

                            processed.push(Input::Text(result));
                        }
                        Err(e) => {
                            // Check if this is a "table not found" error
                            let error_msg = e.to_string();
                            if error_msg.contains("not found") {
                                tracing::warn!(
                                    table_name = %table_name,
                                    "Table not found, treating as empty result for optional input"
                                );
                                // Treat as empty result - don't add to processed inputs
                                // This allows optional table references in first carousel iteration
                            } else {
                                tracing::error!(
                                    table_name = %table_name,
                                    error = %e,
                                    "Table query failed"
                                );
                                return Err(botticelli_error::NarrativeError::new(
                                    botticelli_error::NarrativeErrorKind::TableQueryFailed(
                                        format!("Table query '{}' failed: {}", table_name, e),
                                    ),
                                )
                                .into());
                            }
                        }
                    }
                }

                Input::Narrative { name, path, .. } => {
                    tracing::debug!(
                        name = %name,
                        path = ?path,
                        "Processing narrative reference input - executing nested narrative"
                    );

                    // Resolve the path (if None, use name.toml)
                    // Add .toml extension if not present
                    let mut narrative_path = path
                        .as_ref()
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| name.to_string());

                    if !narrative_path.ends_with(".toml") {
                        narrative_path.push_str(".toml");
                    }

                    // Resolve path relative to parent narrative's directory
                    let resolved_path = if std::path::Path::new(&narrative_path).is_absolute() {
                        std::path::PathBuf::from(&narrative_path)
                    } else if let Some(parent_path) = narrative.source_path() {
                        parent_path
                            .parent()
                            .map(|p| p.join(&narrative_path))
                            .unwrap_or_else(|| std::path::PathBuf::from(&narrative_path))
                    } else {
                        std::path::PathBuf::from(&narrative_path)
                    };

                    tracing::info!(
                        name = %name,
                        path = %resolved_path.display(),
                        exists = %resolved_path.exists(),
                        "Loading nested narrative"
                    );

                    // Load the nested narrative from file
                    let nested_narrative =
                        crate::Narrative::from_file(&resolved_path).map_err(|e| {
                            tracing::error!(
                                name = %name,
                                path = %resolved_path.display(),
                                error = %e,
                                "Failed to load nested narrative"
                            );
                            botticelli_error::NarrativeError::new(
                                botticelli_error::NarrativeErrorKind::NestedNarrativeLoadFailed(
                                    format!(
                                        "Failed to load nested narrative '{}' from '{}': {}",
                                        name,
                                        resolved_path.display(),
                                        e
                                    ),
                                ),
                            )
                        })?;

                    tracing::info!(
                        name = %name,
                        acts = nested_narrative.act_names().len(),
                        "Executing nested narrative"
                    );

                    // Execute the nested narrative recursively
                    // Use Box::pin to avoid infinite sized future in recursive async function
                    let nested_execution = Box::pin(self.execute(&nested_narrative)).await
                        .map_err(|e| {
                            tracing::error!(
                                name = %name,
                                error = %e,
                                "Nested narrative execution failed"
                            );
                            botticelli_error::NarrativeError::new(
                                botticelli_error::NarrativeErrorKind::NestedNarrativeExecutionFailed(
                                    format!("Nested narrative '{}' execution failed: {}", name, e)
                                ),
                            )
                        })?;

                    tracing::info!(
                        name = %name,
                        acts_executed = nested_execution.act_executions.len(),
                        "Nested narrative execution completed"
                    );

                    // Note: We don't include the nested narrative's output in processed inputs
                    // The nested narrative is executed for its side effects (e.g., populating tables)
                    // If you want to include the output, uncomment the lines below:
                    // let final_output = nested_execution.act_executions.last()
                    //     .map(|a| a.output.clone())
                    //     .unwrap_or_default();
                    // processed.push(Input::Text(final_output));
                }

                // Pass through all other input types unchanged
                other => {
                    processed.push(other.clone());
                }
            }
        }

        tracing::Span::current().record("bot_commands", bot_command_count);
        tracing::Span::current().record("tables", table_count);

        Ok((processed, last_bot_command_result))
    }
}

/// Extract text content from LLM outputs.
///
/// Concatenates all text outputs with newlines between them.
fn extract_text_from_outputs(outputs: &[Output]) -> BotticelliResult<String> {
    let mut texts = Vec::new();

    for output in outputs {
        if let Output::Text(text) = output {
            texts.push(text.clone());
        }
    }

    if texts.is_empty() {
        Ok(String::new())
    } else {
        Ok(texts.join("\n"))
    }
}

/// Resolve template placeholders in a string using act execution history and state.
///
/// Supports:
/// - `{{previous}}` - Content from the immediately previous act
/// - `{{act_name}}` - Content from a specific named act
/// - `${state:key}` - Value from persistent state
///
/// # Errors
///
/// Returns error if:
/// - Referenced act doesn't exist
/// - Referenced act hasn't executed yet
/// - Template syntax is malformed
/// - State key doesn't exist
fn resolve_template(
    template: &str,
    act_executions: &[ActExecution],
    current_index: usize,
    state_manager: Option<&StateManager>,
) -> BotticelliResult<String> {
    let mut result = template.to_string();

    // Find all {{...}} or ${...} patterns
    let re = regex::Regex::new(r"(?:\{\{([^}]+)\}\}|\$\{([^}]+)\})").map_err(|e| {
        botticelli_error::NarrativeError::new(botticelli_error::NarrativeErrorKind::TemplateError(
            format!("Invalid template regex: {}", e),
        ))
    })?;

    for cap in re.captures_iter(template) {
        let placeholder = &cap[0]; // Full match like {{previous}}, ${act_name.field}, etc.
        // Get the reference from whichever capture group matched (group 1 for {{...}}, group 2 for ${...})
        let reference = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().trim())
            .ok_or_else(|| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(format!(
                        "Failed to extract reference from placeholder: {}",
                        placeholder
                    )),
                )
            })?;

        let replacement = if reference.starts_with("state:") {
            // State reference like "${state:channel_id}" or "${state:discord.channels.create.channel_id}"
            let state_key = reference.strip_prefix("state:").unwrap();

            let state_mgr = state_manager.ok_or_else(|| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(format!(
                        "State reference '{}' requires state_manager to be configured",
                        reference
                    )),
                )
            })?;

            // Try to load global state
            let state = state_mgr
                .load(&crate::state::StateScope::Global)
                .map_err(|e| {
                    botticelli_error::NarrativeError::new(
                        botticelli_error::NarrativeErrorKind::TemplateError(format!(
                            "Failed to load state: {}",
                            e
                        )),
                    )
                })?;

            state
                .get(state_key)
                .ok_or_else(|| {
                    // Provide helpful error with available keys
                    let available_keys: Vec<_> = state.keys().collect();
                    botticelli_error::NarrativeError::new(
                        botticelli_error::NarrativeErrorKind::TemplateError(format!(
                            "State key '{}' not found. Available keys: {}",
                            state_key,
                            if available_keys.is_empty() {
                                "none".to_string()
                            } else {
                                available_keys.join(", ")
                            }
                        )),
                    )
                })?
                .to_string()
        } else if reference.starts_with("env:") {
            // Environment variable reference like "${env:TEST_GUILD_ID}"
            let env_var = reference.strip_prefix("env:").unwrap();

            std::env::var(env_var).map_err(|e| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(format!(
                        "Environment variable '{}' not found: {}",
                        env_var, e
                    )),
                )
            })?
        } else if reference == "previous" {
            // Get previous act
            if current_index == 0 {
                return Err(botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(
                        "Cannot reference {{previous}} in first act".to_string(),
                    ),
                )
                .into());
            }
            act_executions[current_index - 1].response.clone()
        } else if reference.contains('.') {
            // JSON path reference like "act_name.field" or "act_name.field.subfield"
            let parts: Vec<&str> = reference.splitn(2, '.').collect();
            let act_name = parts[0];
            let json_path = parts[1];

            // Find the act
            let act_exec = act_executions
                .iter()
                .find(|exec| exec.act_name == act_name)
                .ok_or_else(|| {
                    botticelli_error::NarrativeError::new(
                        botticelli_error::NarrativeErrorKind::TemplateError(format!(
                            "Referenced act '{}' not found in execution history",
                            act_name
                        )),
                    )
                })?;

            // Try to parse response as JSON and navigate path
            let json_value: JsonValue = serde_json::from_str(&act_exec.response).map_err(|e| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(format!(
                        "Act '{}' response is not valid JSON: {}",
                        act_name, e
                    )),
                )
            })?;

            // Navigate JSON path
            let mut current = &json_value;
            for segment in json_path.split('.') {
                current = current.get(segment).ok_or_else(|| {
                    botticelli_error::NarrativeError::new(
                        botticelli_error::NarrativeErrorKind::TemplateError(format!(
                            "JSON path '{}' not found in act '{}'",
                            json_path, act_name
                        )),
                    )
                })?;
            }

            // Convert value to string
            match current {
                JsonValue::String(s) => s.clone(),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Null => "null".to_string(),
                _ => serde_json::to_string(current).map_err(|e| {
                    botticelli_error::NarrativeError::new(
                        botticelli_error::NarrativeErrorKind::TemplateError(format!(
                            "Failed to serialize JSON value: {}",
                            e
                        )),
                    )
                })?,
            }
        } else {
            // Get named act
            act_executions
                .iter()
                .find(|exec| exec.act_name == reference)
                .map(|exec| exec.response.clone())
                .ok_or_else(|| {
                    botticelli_error::NarrativeError::new(
                        botticelli_error::NarrativeErrorKind::TemplateError(format!(
                            "Referenced act '{}' not found in execution history",
                            reference
                        )),
                    )
                })?
        };

        result = result.replace(placeholder, &replacement);
    }

    Ok(result)
}
