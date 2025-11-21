//! Narrative execution logic.
//!
//! This module provides the executor that processes multi-act narratives
//! by calling LLM APIs in sequence, passing context between acts.

use crate::{CarouselResult, CarouselState, NarrativeProvider, ProcessorContext, ProcessorRegistry};
use botticelli_core::{GenerateRequest, Input, Message, Output, Role};
use botticelli_error::BotticelliResult;
use botticelli_interface::{ActExecution, BotticelliDriver, NarrativeExecution, TableQueryRegistry};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

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
pub struct NarrativeExecutor<D: BotticelliDriver> {
    driver: D,
    processor_registry: Option<ProcessorRegistry>,
    bot_registry: Option<Box<dyn BotCommandRegistry>>,
    table_registry: Option<Box<dyn TableQueryRegistry>>,
}

impl<D: BotticelliDriver> NarrativeExecutor<D> {
    /// Create a new narrative executor with the given LLM driver.
    pub fn new(driver: D) -> Self {
        Self {
            driver,
            processor_registry: None,
            bot_registry: None,
            table_registry: None,
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
    #[tracing::instrument(skip(self, narrative), fields(narrative_name = narrative.name(), act_count = narrative.act_names().len()))]
    pub async fn execute<N: NarrativeProvider>(
        &self,
        narrative: &N,
    ) -> BotticelliResult<NarrativeExecution> {
        let mut act_executions = Vec::new();
        let mut conversation_history: Vec<Message> = Vec::new();

        for (sequence_number, act_name) in narrative.act_names().iter().enumerate() {
            // Get the configuration for this act
            let config = narrative
                .get_act_config(act_name)
                .expect("NarrativeProvider should ensure all acts exist");

            // Process inputs (execute bot commands, query tables, etc.)
            let processed_inputs = self.process_inputs(&config.inputs).await?;

            // Build the request with conversation history + processed inputs
            conversation_history.push(Message {
                role: Role::User,
                content: processed_inputs.clone(),
            });

            let request = GenerateRequest {
                messages: conversation_history.clone(),
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                model: config.model.clone(),
            };

            // Call the LLM
            let response = self.driver.generate(&request).await?;

            // Extract text from response
            let response_text = extract_text_from_outputs(&response.outputs)?;

            // Create the act execution (store processed inputs)
            let act_execution = ActExecution {
                act_name: act_name.clone(),
                inputs: processed_inputs.clone(),
                model: config.model,
                temperature: config.temperature,
                max_tokens: config.max_tokens,
                response: response_text.clone(),
                sequence_number,
            };

            // Process with registered processors
            if let Some(registry) = &self.processor_registry {
                tracing::info!(
                    act = %act_name,
                    processors = registry.len(),
                    "Processing act with registered processors"
                );

                // Build processor context
                let context = ProcessorContext {
                    execution: &act_execution,
                    narrative_metadata: narrative.metadata(),
                    narrative_name: narrative.name(),
                };

                if let Err(e) = registry.process(&context).await {
                    tracing::error!(
                        act = %act_name,
                        error = %e,
                        "Act processing failed, continuing execution"
                    );
                    // Note: We don't fail the entire narrative on processor errors
                    // The user still gets the execution results
                }
            }

            // Store the act execution
            act_executions.push(act_execution);

            // Add the assistant's response to conversation history for the next act
            conversation_history.push(Message {
                role: Role::Assistant,
                content: vec![Input::Text(response_text)],
            });
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
    pub async fn execute_carousel<N: NarrativeProvider>(
        &self,
        narrative: &N,
    ) -> BotticelliResult<CarouselResult> {
        // Get carousel config from narrative
        let carousel_config = narrative
            .carousel_config()
            .ok_or_else(|| {
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
        let mut state = CarouselState::new(
            carousel_config.clone(),
            *self.driver.rate_limits(),
        );

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
    #[tracing::instrument(
        skip(self, inputs),
        fields(
            input_count = inputs.len(),
            bot_commands = 0,
            tables = 0
        )
    )]
    async fn process_inputs(&self, inputs: &[Input]) -> BotticelliResult<Vec<Input>> {
        let mut processed = Vec::new();
        let mut bot_command_count = 0;
        let mut table_count = 0;

        for input in inputs {
            match input {
                Input::BotCommand {
                    platform,
                    command,
                    args,
                    required,
                    cache_duration: _,
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

                    match registry.execute(platform, command, args).await {
                        Ok(result) => {
                            // Convert JSON result to pretty-printed text for LLM
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
                    alias: _,
                    format,
                    sample: _,
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

                    match registry
                        .query_table(
                            table_name,
                            columns.as_deref(),
                            where_clause.as_deref(),
                            *limit,
                            *offset,
                            order_by.as_deref(),
                            format_str,
                        )
                        .await
                    {
                        Ok(result) => {
                            tracing::info!(
                                table_name = %table_name,
                                result_length = result.len(),
                                "Table query executed successfully"
                            );

                            processed.push(Input::Text(result));
                        }
                        Err(e) => {
                            tracing::error!(
                                table_name = %table_name,
                                error = %e,
                                "Table query failed"
                            );
                            return Err(botticelli_error::NarrativeError::new(
                                botticelli_error::NarrativeErrorKind::TableQueryFailed(format!(
                                    "Table query '{}' failed: {}",
                                    table_name, e
                                )),
                            )
                            .into());
                        }
                    }
                }

                Input::Narrative { name, path } => {
                    tracing::debug!(
                        name = %name,
                        path = ?path,
                        "Processing narrative reference input"
                    );
                    
                    // TODO: Implement narrative composition
                    // For now, return a placeholder message
                    let msg = format!("[Narrative composition not yet implemented: {}]", name);
                    tracing::warn!(name = %name, "Narrative composition feature pending");
                    processed.push(Input::Text(msg));
                }

                // Pass through all other input types unchanged
                other => {
                    processed.push(other.clone());
                }
            }
        }

        tracing::Span::current().record("bot_commands", bot_command_count);
        tracing::Span::current().record("tables", table_count);

        Ok(processed)
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
