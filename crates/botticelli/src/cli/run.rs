//! Narrative execution command handler.

use botticelli::BotticelliResult;
#[cfg(feature = "gemini")]
use botticelli_core::BudgetConfig;
#[cfg(feature = "gemini")]
use std::path::{Path, PathBuf};

/// Source specification for loading a narrative.
///
/// Encapsulates the file path and optional narrative name for multi-narrative files.
///
/// Available with the `gemini` feature.
#[cfg(feature = "gemini")]
#[derive(Debug, Clone)]
pub struct NarrativeSource {
    path: PathBuf,
    name: Option<String>,
}

#[cfg(feature = "gemini")]
impl NarrativeSource {
    /// Create a new narrative source.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the narrative TOML file
    /// * `name` - Optional specific narrative name for multi-narrative files
    pub fn new(path: impl Into<PathBuf>, name: Option<String>) -> Self {
        Self {
            path: path.into(),
            name,
        }
    }

    /// Get the narrative file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the optional narrative name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// Execution options for narrative running.
///
/// Configures save behavior, Discord processing, and state persistence.
///
/// Available with the `gemini` feature.
#[cfg(feature = "gemini")]
#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    save: bool,
    #[cfg(feature = "discord")]
    process_discord: bool,
    #[cfg(feature = "database")]
    state_dir: Option<PathBuf>,
}

#[cfg(feature = "gemini")]
impl ExecutionOptions {
    /// Create a new execution options builder.
    pub fn builder() -> ExecutionOptionsBuilder {
        ExecutionOptionsBuilder::default()
    }

    /// Whether to save execution results to the database.
    pub fn save(&self) -> bool {
        self.save
    }

    /// Whether to process Discord infrastructure (guilds, channels, etc.).
    ///
    /// Available with the `discord` feature.
    #[cfg(feature = "discord")]
    pub fn process_discord(&self) -> bool {
        self.process_discord
    }

    /// Get the state directory for persistent storage.
    ///
    /// Available with the `database` feature.
    #[cfg(feature = "database")]
    pub fn state_dir(&self) -> Option<&Path> {
        self.state_dir.as_deref()
    }
}

/// Builder for execution options.
///
/// Provides a fluent interface for constructing `ExecutionOptions`.
///
/// Available with the `gemini` feature.
#[cfg(feature = "gemini")]
#[derive(Debug, Clone, Default)]
pub struct ExecutionOptionsBuilder {
    save: bool,
    #[cfg(feature = "discord")]
    process_discord: bool,
    #[cfg(feature = "database")]
    state_dir: Option<PathBuf>,
}

#[cfg(feature = "gemini")]
impl ExecutionOptionsBuilder {
    /// Set whether to save execution results to the database.
    pub fn save(mut self, save: bool) -> Self {
        self.save = save;
        self
    }

    /// Set whether to process Discord infrastructure.
    ///
    /// Available with the `discord` feature.
    #[cfg(feature = "discord")]
    pub fn process_discord(mut self, process_discord: bool) -> Self {
        self.process_discord = process_discord;
        self
    }

    /// Set the state directory for persistent storage.
    ///
    /// Available with the `database` feature.
    #[cfg(feature = "database")]
    pub fn state_dir(mut self, state_dir: Option<PathBuf>) -> Self {
        self.state_dir = state_dir;
        self
    }

    /// Build the execution options.
    pub fn build(self) -> ExecutionOptions {
        ExecutionOptions {
            save: self.save,
            #[cfg(feature = "discord")]
            process_discord: self.process_discord,
            #[cfg(feature = "database")]
            state_dir: self.state_dir,
        }
    }
}

/// Execute a narrative from a TOML file.
///
/// # Arguments
///
/// * `source` - Narrative source specification (path and optional name)
/// * `options` - Execution options (save, Discord processing, state directory)
/// * `budget_overrides` - Optional budget multipliers to override configuration
#[cfg(feature = "gemini")]
pub async fn run_narrative(
    source: &NarrativeSource,
    options: &ExecutionOptions,
    budget_overrides: Option<&BudgetConfig>,
) -> BotticelliResult<()> {
    use botticelli::{GeminiClient, NarrativeExecutor};

    #[cfg(not(feature = "database"))]

    tracing::info!(
        path = %source.path().display(),
        narrative_name = ?source.name(),
        "Loading narrative"
    );

    // Load and parse the narrative TOML file
    // Use MultiNarrative if a name is provided (enables composition), otherwise single Narrative
    #[cfg(feature = "database")]
    let narrative: Box<dyn botticelli::NarrativeProvider> = {
        let mut conn = botticelli::establish_connection()?;

        if let Some(name) = source.name() {
            // Load as MultiNarrative for composition support
            Box::new(botticelli::MultiNarrative::from_file_with_db(
                source.path(),
                name,
                &mut conn,
            )?)
        } else {
            // Load as single Narrative for backwards compatibility
            let content = std::fs::read_to_string(source.path()).map_err(|e| {
                botticelli::NarrativeError::new(botticelli::NarrativeErrorKind::FileRead(
                    e.to_string(),
                ))
            })?;
            let mut narrative = botticelli::Narrative::from_toml_str(&content, None)?;
            narrative.set_source_path(Some(source.path().to_path_buf()));

            // Assemble prompts if template specified
            if narrative.metadata().template().is_some() {
                narrative.assemble_act_prompts(&mut conn)?;
            }

            Box::new(narrative)
        }
    };

    #[cfg(not(feature = "database"))]
    let narrative: Box<dyn botticelli::NarrativeProvider> = {
        if let Some(name) = source.name() {
            // Load as MultiNarrative for composition support
            Box::new(botticelli::MultiNarrative::from_file(source.path(), name)?)
        } else {
            // Load as single Narrative for backwards compatibility
            let content = std::fs::read_to_string(source.path()).map_err(|e| {
                botticelli::NarrativeError::new(botticelli::NarrativeErrorKind::FileRead(
                    e.to_string(),
                ))
            })?;
            let mut narrative = botticelli::Narrative::from_toml_str(&content, None)?;
            narrative.set_source_path(Some(source.path().to_path_buf()));
            Box::new(narrative)
        }
    };

    tracing::info!(
        name = %narrative.metadata().name(),
        acts = narrative.act_names().len(),
        "Narrative loaded"
    );

    // Build budget config from CLI args, carousel config, narrative metadata, and config file
    // Priority: CLI > Carousel > Narrative > Config File > Default (1.0)
    let budget = {
        use botticelli_core::BudgetConfig;
        use botticelli_rate_limit::BotticelliConfig;

        let mut budget = BudgetConfig::default();

        // Apply config file default budget if present (lowest priority after default)
        if let Ok(config) = BotticelliConfig::load()
            && let Some(config_budget) = config.budget
        {
            budget = budget.merge(&config_budget);
            tracing::debug!(
                rpm = config_budget.rpm_multiplier(),
                tpm = config_budget.tpm_multiplier(),
                rpd = config_budget.rpd_multiplier(),
                "Loaded default budget from botticelli.toml"
            );
        }

        // Apply narrative-level budget if present
        if let Some(narrative_budget) = narrative.metadata().budget() {
            budget = budget.merge(narrative_budget);
        }

        // Apply carousel-level budget if present
        if let Some(carousel) = narrative.metadata().carousel()
            && let Some(carousel_budget) = carousel.budget()
        {
            budget = budget.merge(carousel_budget);
        }

        // Apply CLI overrides if present (highest priority)
        if let Some(cli_budget) = budget_overrides {
            budget = budget.merge(cli_budget);
        }

        // Validate the final budget
        budget.validate().map_err(|e| {
            botticelli::NarrativeError::new(botticelli::NarrativeErrorKind::ConfigurationError(e))
        })?;

        // Log if throttling is active
        if budget.rpm_multiplier() < &1.0
            || budget.tpm_multiplier() < &1.0
            || budget.rpd_multiplier() < &1.0
        {
            tracing::info!(
                rpm = budget.rpm_multiplier(),
                tpm = budget.tpm_multiplier(),
                rpd = budget.rpd_multiplier(),
                "Applying budget multipliers"
            );
        }

        budget
    };

    // Create Gemini client with budget-adjusted rate limits
    let client = {
        use botticelli_rate_limit::{BotticelliConfig, TierConfig};

        // Load base tier from config
        let tier_config = BotticelliConfig::load()
            .ok()
            .and_then(|config| config.get_tier("gemini", None))
            .unwrap_or_else(|| {
                // Default Free tier if no config
                TierConfig {
                    name: "Free".to_string(),
                    rpm: Some(10),
                    tpm: Some(250_000),
                    rpd: Some(250),
                    max_concurrent: Some(1),
                    daily_quota_usd: None,
                    cost_per_million_input_tokens: Some(0.0),
                    cost_per_million_output_tokens: Some(0.0),
                    models: std::collections::HashMap::new(),
                }
            });

        // Apply budget multipliers to create adjusted tier
        let adjusted_tier = if budget.rpm_multiplier() < &1.0
            || budget.tpm_multiplier() < &1.0
            || budget.rpd_multiplier() < &1.0
        {
            // Apply multipliers to rate limits
            TierConfig {
                name: format!(
                    "{} ({}x)",
                    tier_config.name,
                    budget.rpm_multiplier().min(*budget.rpd_multiplier())
                ),
                rpm: tier_config.rpm.map(|r| budget.apply_rpm(r as u64) as u32),
                tpm: tier_config.tpm.map(|t| budget.apply_tpm(t)),
                rpd: tier_config.rpd.map(|r| budget.apply_rpd(r as u64) as u32),
                max_concurrent: tier_config.max_concurrent,
                daily_quota_usd: tier_config.daily_quota_usd,
                cost_per_million_input_tokens: tier_config.cost_per_million_input_tokens,
                cost_per_million_output_tokens: tier_config.cost_per_million_output_tokens,
                models: tier_config.models.clone(),
            }
        } else {
            tier_config
        };

        GeminiClient::new_with_tier(Some(Box::new(adjusted_tier)))?
    };

    // Create executor with content generation processor and table registry
    let executor = {
        #[cfg(feature = "database")]
        {
            use botticelli::ProcessorRegistry;
            use botticelli_database::{
                DatabaseTableQueryRegistry, TableQueryExecutor, create_pool,
            };
            use botticelli_narrative::{ContentGenerationProcessor, StorageActor};
            use std::sync::{Arc, Mutex};

            // Create database connection for table queries
            let table_conn = botticelli::establish_connection()?;
            let table_executor = TableQueryExecutor::new(Arc::new(Mutex::new(table_conn)));
            let table_registry = DatabaseTableQueryRegistry::new(table_executor);

            // Start storage actor with Ractor
            tracing::info!("Starting storage actor");
            let pool = create_pool()?;

            let actor = StorageActor::new(pool.clone());
            let (actor_ref, _handle) =
                ractor::Actor::spawn(None, actor, pool).await.map_err(|e| {
                    botticelli_error::BackendError::new(format!(
                        "Failed to spawn storage actor: {}",
                        e
                    ))
                })?;

            tracing::info!("Storage actor started");

            // Create content generation processor with storage actor
            let processor = ContentGenerationProcessor::new(actor_ref);

            let mut registry = ProcessorRegistry::new();
            registry.register(Box::new(processor));

            // Build executor with processors and table registry
            tracing::info!("Configuring executor with table registry");
            let mut executor = NarrativeExecutor::with_processors(client, registry);
            executor = executor.with_table_registry(Box::new(table_registry));
            tracing::info!("Table registry configured");

            // Configure bot command registry (requires discord feature for social integration)
            #[cfg(all(feature = "database", feature = "discord"))]
            {
                use botticelli_social::{BotCommandRegistryImpl, DatabaseCommandExecutor};

                tracing::info!("Configuring bot command registry");
                let mut bot_registry = BotCommandRegistryImpl::new();

                // Always register database executor
                let database_executor = DatabaseCommandExecutor::new();
                bot_registry.register(database_executor);
                tracing::info!("Database command executor registered");

                // Configure Discord bot executor if feature enabled and requested
                #[cfg(feature = "discord")]
                if options.process_discord() {
                    use botticelli_social::DiscordCommandExecutor;
                    use std::env;

                    if let Ok(token) = env::var("DISCORD_TOKEN") {
                        tracing::info!("Configuring Discord bot executor");
                        let discord_executor = DiscordCommandExecutor::new(token);
                        bot_registry.register(discord_executor);
                        tracing::info!("Discord bot executor registered");
                    } else {
                        tracing::warn!("DISCORD_TOKEN not set, Discord commands will fail");
                    }
                }

                #[cfg(not(feature = "discord"))]
                if options.process_discord() {
                    tracing::warn!("Discord feature not enabled, Discord commands will fail");
                }

                executor = executor.with_bot_registry(Box::new(bot_registry));
                tracing::info!("Bot command registry configured");
            }

            // Configure state manager if state_dir provided
            if let Some(dir) = options.state_dir() {
                tracing::info!(state_dir = %dir.display(), "Configuring state manager");
                use botticelli_narrative::StateManager;
                let state_mgr = StateManager::new(dir)?;
                executor = executor.with_state_manager(state_mgr);
                tracing::info!("State manager configured");
            }

            executor
        }

        #[cfg(not(feature = "database"))]
        {
            #[cfg(feature = "discord")]
            if options.process_discord() {
                tracing::warn!("Discord processing requires database feature");
            }
            NarrativeExecutor::new(client)
        }
    };

    // Execute the narrative (with carousel if configured)
    tracing::info!("Executing narrative");

    if narrative.carousel_config().is_some() {
        tracing::info!("Executing narrative in carousel mode");
        let carousel_result = executor.execute_carousel(narrative.as_ref()).await?;

        tracing::info!(
            iterations_attempted = carousel_result.iterations_attempted(),
            successful = carousel_result.successful_iterations(),
            failed = carousel_result.failed_iterations(),
            "Carousel execution completed"
        );

        // Print carousel summary
        println!("\nCarousel Execution Summary:");
        println!("============================");
        println!("Narrative: {}", narrative.metadata().name());
        println!(
            "Iterations attempted: {}",
            carousel_result.iterations_attempted()
        );
        println!(
            "Successful iterations: {}",
            carousel_result.successful_iterations()
        );
        println!("Failed iterations: {}", carousel_result.failed_iterations());
        println!("Completed: {}", carousel_result.completed());
        println!("Budget exhausted: {}", carousel_result.budget_exhausted());
        println!();

        return Ok(());
    }

    let execution = executor.execute(narrative.as_ref()).await?;

    tracing::info!(
        acts_completed = execution.act_executions.len(),
        "Narrative execution completed"
    );

    // Save to database if requested
    if options.save() {
        #[cfg(feature = "database")]
        {
            use botticelli::{
                NarrativeRepository, PostgresNarrativeRepository, establish_connection,
            };
            use botticelli_storage::FileSystemStorage;
            use std::sync::Arc;

            let conn = establish_connection()?;
            let storage_dir = dirs::data_dir()
                .expect("Could not determine data directory")
                .join("botticelli")
                .join("storage");
            let storage = Arc::new(FileSystemStorage::new(storage_dir)?);
            let repo = PostgresNarrativeRepository::new(conn, storage);

            let exec_id = repo.save_execution(&execution).await?;
            tracing::info!(execution_id = exec_id, "Execution saved to database");
        }

        #[cfg(not(feature = "database"))]
        {
            tracing::warn!("Database feature not enabled, ignoring --save flag");
        }
    }

    // Print execution summary
    println!("\nNarrative Execution Summary:");
    println!("============================");
    println!("Narrative: {}", execution.narrative_name);
    println!("Acts completed: {}", execution.act_executions.len());
    println!();

    for act in &execution.act_executions {
        println!("Act {}: {}", act.sequence_number + 1, act.act_name);
        println!("  Response length: {} characters", act.response.len());
        if let Some(model) = &act.model {
            println!("  Model: {}", model);
        }
        println!();
    }

    Ok(())
}

#[cfg(not(feature = "gemini"))]
pub async fn run_narrative() -> BotticelliResult<()> {
    eprintln!("Error: Gemini feature not enabled. Rebuild with --features gemini");
    std::process::exit(1);
}
