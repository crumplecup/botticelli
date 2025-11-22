//! Narrative execution logic.
//!
//! This module provides the executor that processes multi-act narratives
//! by calling LLM APIs in sequence, passing context between acts.

use crate::{CarouselResult, CarouselState, NarrativeProvider, ProcessorContext, ProcessorRegistry, StateManager};
use botticelli_core::{GenerateRequest, Input, Message, MessageBuilder, Output, Role};
use botticelli_error::{BotticelliError, BotticelliResult, NarrativeError, NarrativeErrorKind};
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
            // Pass execution history for template resolution
            let (processed_inputs, bot_command_result) = self.process_inputs(narrative, config.inputs(), &act_executions, sequence_number).await?;

            // Check if this is an action-only act (no text inputs from TOML that need LLM processing)
            // Bot command results in processed_inputs should NOT trigger LLM calls
            let has_text_prompt = config.inputs().iter().any(|input| matches!(input, Input::Text(text) if !text.trim().is_empty()));
            
            let (response_text, model, temperature, max_tokens) = if has_text_prompt {
                // This act needs an LLM response
                // Build the request with conversation history + processed inputs
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
                        BotticelliError::from(NarrativeError::new(NarrativeErrorKind::FileRead(format!(
                            "Failed to build request: {}",
                            e
                        ))))
                    })?;

                // Call the LLM
                let response = self.driver.generate(&request).await?;

                // Extract text from response
                let response_text = extract_text_from_outputs(&response.outputs)?;
                
                (response_text, model, temperature, max_tokens)
            } else {
                // Action-only act - no LLM call needed
                tracing::debug!(
                    act = %act_name,
                    "Skipping LLM call for action-only act"
                );
                // Use bot command result as response if available, otherwise generic success message
                let response_text = if let Some(result) = bot_command_result {
                    serde_json::to_string(&result).unwrap_or_else(|_| "Action completed successfully".to_string())
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
    async fn process_inputs<N: NarrativeProvider>(
        &self,
        narrative: &N,
        inputs: &[Input],
        act_executions: &[ActExecution],
        current_index: usize,
    ) -> BotticelliResult<(Vec<Input>, Option<JsonValue>)> {
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

                    // Resolve templates in bot command arguments
                    let mut resolved_args = args.clone();
                    for (_key, value) in resolved_args.iter_mut() {
                        if let JsonValue::String(s) = value {
                            *s = resolve_template(s, act_executions, current_index, self.state_manager.as_ref())?;
                        }
                    }

                    match registry.execute(platform, command, &resolved_args).await {
                        Ok(result) => {
                            // Store bot command result for potential use as act output
                            last_bot_command_result = Some(result.clone());
                            
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
                                    botticelli_error::NarrativeErrorKind::TableQueryFailed(format!(
                                        "Table query '{}' failed: {}",
                                        table_name, e
                                    )),
                                )
                                .into());
                            }
                        }
                    }
                }

                Input::Narrative { name, path } => {
                    tracing::debug!(
                        name = %name,
                        path = ?path,
                        "Processing narrative reference input - executing nested narrative"
                    );
                    
                    // Resolve the path (if None, use name.toml)
                    // Add .toml extension if not present
                    let mut narrative_path = path.as_ref()
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| name.to_string());
                    
                    if !narrative_path.ends_with(".toml") {
                        narrative_path.push_str(".toml");
                    }
                    
                    // Resolve path relative to parent narrative's directory
                    let resolved_path = if std::path::Path::new(&narrative_path).is_absolute() {
                        std::path::PathBuf::from(&narrative_path)
                    } else if let Some(parent_path) = narrative.source_path() {
                        parent_path.parent()
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
                    let nested_narrative = crate::Narrative::from_file(&resolved_path)
                        .map_err(|e| {
                            tracing::error!(
                                name = %name,
                                path = %resolved_path.display(),
                                error = %e,
                                "Failed to load nested narrative"
                            );
                            botticelli_error::NarrativeError::new(
                                botticelli_error::NarrativeErrorKind::NestedNarrativeLoadFailed(
                                    format!("Failed to load nested narrative '{}' from '{}': {}", name, resolved_path.display(), e)
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
        botticelli_error::NarrativeError::new(
            botticelli_error::NarrativeErrorKind::TemplateError(
                format!("Invalid template regex: {}", e),
            ),
        )
    })?;
    
    for cap in re.captures_iter(template) {
        let placeholder = &cap[0]; // Full match like {{previous}}, ${act_name.field}, etc.
        // Get the reference from whichever capture group matched (group 1 for {{...}}, group 2 for ${...})
        let reference = cap.get(1).or_else(|| cap.get(2))
            .map(|m| m.as_str().trim())
            .ok_or_else(|| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(
                        format!("Failed to extract reference from placeholder: {}", placeholder),
                    ),
                )
            })?;
        
        let replacement = if reference.starts_with("state:") {
            // State reference like "${state:channel_id}" or "${state:discord.channels.create.channel_id}"
            let state_key = reference.strip_prefix("state:").unwrap();
            
            let state_mgr = state_manager.ok_or_else(|| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(
                        format!("State reference '{}' requires state_manager to be configured", reference),
                    ),
                )
            })?;
            
            // Try to load global state
            let state = state_mgr.load(&crate::state::StateScope::Global).map_err(|e| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(
                        format!("Failed to load state: {}", e),
                    ),
                )
            })?;
            
            state.get(state_key).ok_or_else(|| {
                // Provide helpful error with available keys
                let available_keys: Vec<_> = state.keys().collect();
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(
                        format!(
                            "State key '{}' not found. Available keys: {}",
                            state_key,
                            if available_keys.is_empty() {
                                "none".to_string()
                            } else {
                                available_keys.join(", ")
                            }
                        ),
                    ),
                )
            })?.to_string()
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
                        botticelli_error::NarrativeErrorKind::TemplateError(
                            format!("Referenced act '{}' not found in execution history", act_name),
                        ),
                    )
                })?;
            
            // Try to parse response as JSON and navigate path
            let json_value: JsonValue = serde_json::from_str(&act_exec.response).map_err(|e| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::TemplateError(
                        format!("Act '{}' response is not valid JSON: {}", act_name, e),
                    ),
                )
            })?;
            
            // Navigate JSON path
            let mut current = &json_value;
            for segment in json_path.split('.') {
                current = current.get(segment).ok_or_else(|| {
                    botticelli_error::NarrativeError::new(
                        botticelli_error::NarrativeErrorKind::TemplateError(
                            format!("JSON path '{}' not found in act '{}'", json_path, act_name),
                        ),
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
                        botticelli_error::NarrativeErrorKind::TemplateError(
                            format!("Failed to serialize JSON value: {}", e),
                        ),
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
                        botticelli_error::NarrativeErrorKind::TemplateError(
                            format!("Referenced act '{}' not found in execution history", reference),
                        ),
                    )
                })?
        };
        
        result = result.replace(placeholder, &replacement);
    }
    
    Ok(result)
}
