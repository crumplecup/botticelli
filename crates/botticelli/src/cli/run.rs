//! Narrative execution command handler.

use botticelli::BotticelliResult;
use std::path::Path;

/// Execute a narrative from a TOML file.
///
/// # Arguments
///
/// * `narrative_path` - Path to the narrative TOML file
/// * `narrative_name` - Optional specific narrative name (for multi-narrative files)
/// * `save` - Whether to save execution results to the database
/// * `process_discord` - Whether to process Discord infrastructure
/// * `state_dir` - Optional directory for persistent state storage
#[cfg(feature = "gemini")]
pub async fn run_narrative(
    narrative_path: &Path,
    narrative_name: Option<&str>,
    save: bool,
    process_discord: bool,
    #[cfg(feature = "database")] state_dir: Option<&Path>,
) -> BotticelliResult<()> {
    use botticelli::{GeminiClient, NarrativeExecutor, NarrativeProvider};

    #[cfg(not(feature = "database"))]
    use botticelli::Narrative;

    tracing::info!(
        path = %narrative_path.display(),
        narrative_name = ?narrative_name,
        "Loading narrative"
    );

    // Load and parse the narrative TOML file
    // If database feature is enabled, use from_file_with_db to inject schema docs
    #[cfg(feature = "database")]
    let narrative = {
        let mut conn = botticelli::establish_connection()?;

        // Read file and parse with optional narrative name
        let content = std::fs::read_to_string(narrative_path).map_err(|e| {
            botticelli::NarrativeError::new(botticelli::NarrativeErrorKind::FileRead(e.to_string()))
        })?;
        let mut narrative = botticelli::Narrative::from_toml_str(&content, narrative_name)?;
        narrative.set_source_path(Some(narrative_path.to_path_buf()));

        // Assemble prompts if template specified
        if narrative.metadata().template().is_some() {
            narrative.assemble_act_prompts(&mut conn)?;
        }

        narrative
    };

    #[cfg(not(feature = "database"))]
    let narrative = {
        let content = std::fs::read_to_string(narrative_path).map_err(|e| {
            botticelli::NarrativeError::new(botticelli::NarrativeErrorKind::FileRead(e.to_string()))
        })?;
        let mut narrative = Narrative::from_toml_str(&content, narrative_name)?;
        narrative.set_source_path(Some(narrative_path.to_path_buf()));
        narrative
    };

    tracing::info!(
        name = %narrative.metadata().name(),
        acts = narrative.toc().order().len(),
        "Narrative loaded"
    );

    // Create Gemini client (reads API key from GEMINI_API_KEY environment variable)
    let client = GeminiClient::new()?;

    // Create executor with content generation processor and table registry
    let executor = {
        #[cfg(feature = "database")]
        {
            use botticelli::ProcessorRegistry;
            use botticelli_database::{DatabaseTableQueryRegistry, TableQueryExecutor};
            use botticelli_narrative::ContentGenerationProcessor;
            use std::sync::{Arc, Mutex};

            // Create database connection for table queries
            let table_conn = botticelli::establish_connection()?;
            let table_executor = TableQueryExecutor::new(Arc::new(Mutex::new(table_conn)));
            let table_registry = DatabaseTableQueryRegistry::new(table_executor);

            // Create content generation processor
            let proc_conn = botticelli::establish_connection()?;
            let processor = ContentGenerationProcessor::new(Arc::new(Mutex::new(proc_conn)));

            let mut registry = ProcessorRegistry::new();
            registry.register(Box::new(processor));

            // Build executor with processors and table registry
            tracing::info!("Configuring executor with table registry");
            let mut executor = NarrativeExecutor::with_processors(client, registry);
            executor = executor.with_table_registry(Box::new(table_registry));
            tracing::info!("Table registry configured");

            // Configure Discord bot registry if feature enabled and requested
            #[cfg(feature = "discord")]
            if process_discord {
                use botticelli_social::{BotCommandRegistryImpl, DiscordCommandExecutor};
                use std::env;

                if let Ok(token) = env::var("DISCORD_TOKEN") {
                    tracing::info!("Configuring Discord bot registry");
                    let discord_executor = DiscordCommandExecutor::new(token);
                    let mut bot_registry = BotCommandRegistryImpl::new();
                    bot_registry.register(discord_executor);
                    executor = executor.with_bot_registry(Box::new(bot_registry));
                    tracing::info!("Discord bot registry configured");
                } else {
                    tracing::warn!("DISCORD_TOKEN not set, Discord commands will fail");
                }
            }

            #[cfg(not(feature = "discord"))]
            if process_discord {
                tracing::warn!("Discord feature not enabled, Discord commands will fail");
            }

            // Configure state manager if state_dir provided
            if let Some(dir) = state_dir {
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
            if process_discord {
                tracing::warn!("Discord processing requires database feature");
            }
            NarrativeExecutor::new(client)
        }
    };

    // Execute the narrative (with carousel if configured)
    tracing::info!("Executing narrative");

    if narrative.carousel_config().is_some() {
        tracing::info!("Executing narrative in carousel mode");
        let carousel_result = executor.execute_carousel(&narrative).await?;

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

    let execution = executor.execute(&narrative).await?;

    tracing::info!(
        acts_completed = execution.act_executions.len(),
        "Narrative execution completed"
    );

    // Save to database if requested
    if save {
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
pub async fn run_narrative(
    _narrative_path: &Path,
    _narrative_name: Option<&str>,
    _save: bool,
    _process_discord: bool,
    #[cfg(feature = "database")] _state_dir: Option<&Path>,
) -> BotticelliResult<()> {
    eprintln!("Error: Gemini feature not enabled. Rebuild with --features gemini");
    std::process::exit(1);
}
