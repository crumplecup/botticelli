//! Narrative execution command handler.

use botticelli::BotticelliResult;
use std::path::Path;

/// Execute a narrative from a TOML file.
///
/// # Arguments
///
/// * `narrative_path` - Path to the narrative TOML file
/// * `save` - Whether to save execution results to the database
/// * `process_discord` - Whether to process Discord infrastructure
#[cfg(feature = "gemini")]
pub async fn run_narrative(
    narrative_path: &Path,
    save: bool,
    process_discord: bool,
) -> BotticelliResult<()> {
    use botticelli::{GeminiClient, NarrativeExecutor};

    #[cfg(not(feature = "database"))]
    use botticelli::Narrative;

    tracing::info!(path = %narrative_path.display(), "Loading narrative");

    // Load and parse the narrative TOML file
    // If database feature is enabled, use from_file_with_db to inject schema docs
    #[cfg(feature = "database")]
    let narrative = {
        let mut conn = botticelli::establish_connection()?;
        botticelli::Narrative::from_file_with_db(narrative_path, &mut conn)?
    };

    #[cfg(not(feature = "database"))]
    let narrative = Narrative::from_file(narrative_path)?;

    tracing::info!(
        name = %narrative.metadata.name,
        acts = narrative.toc.order.len(),
        "Narrative loaded"
    );

    // Create Gemini client (reads API key from GEMINI_API_KEY environment variable)
    let client = GeminiClient::new()?;

    // Create executor with content generation processor
    let executor = {
        #[cfg(feature = "database")]
        {
            use botticelli::ProcessorRegistry;
            use botticelli_narrative::ContentGenerationProcessor;
            use std::sync::{Arc, Mutex};

            // Create content generation processor if database feature is enabled
            let conn = botticelli::establish_connection()?;
            let processor = ContentGenerationProcessor::new(Arc::new(Mutex::new(conn)));

            let mut registry = ProcessorRegistry::new();
            registry.register(Box::new(processor));

            if process_discord {
                tracing::warn!("Discord-specific processors not yet implemented");
            }

            NarrativeExecutor::with_processors(client, registry)
        }

        #[cfg(not(feature = "database"))]
        {
            if process_discord {
                tracing::warn!("Discord processing requires database feature");
            }
            NarrativeExecutor::new(client)
        }
    };

    // Execute the narrative
    tracing::info!("Executing narrative");
    let execution = executor.execute(&narrative).await?;

    tracing::info!(
        acts_completed = execution.act_executions.len(),
        "Narrative execution completed"
    );

    // Save to database if requested
    if save {
        #[cfg(feature = "database")]
        {
            use botticelli::{establish_connection, PostgresNarrativeRepository, NarrativeRepository};
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
    _save: bool,
    _process_discord: bool,
) -> BotticelliResult<()> {
    eprintln!("Error: Gemini feature not enabled. Rebuild with --features gemini");
    std::process::exit(1);
}
