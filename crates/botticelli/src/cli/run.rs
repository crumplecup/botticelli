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
    use botticelli::{GeminiClient, NarrativeExecutor, Narrative};

    eprintln!("Loading narrative from: {}", narrative_path.display());

    // Load and parse the narrative TOML file
    let narrative = Narrative::from_file(narrative_path)?;

    eprintln!("Narrative '{}' loaded with {} acts", narrative.metadata.name, narrative.toc.order.len());

    // Create Gemini client (reads API key from GEMINI_API_KEY environment variable)
    let client = GeminiClient::new()?;

    // Create executor (Discord processors not yet available)
    let executor = if process_discord {
        eprintln!("Warning: Discord processors not yet implemented, executing without processors");
        NarrativeExecutor::new(client)
    } else {
        NarrativeExecutor::new(client)
    };

    // Execute the narrative
    eprintln!("Executing narrative...");
    let execution = executor.execute(&narrative).await?;

    eprintln!("Narrative execution completed: {} acts", execution.act_executions.len());

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
            eprintln!("Execution saved with ID: {}", exec_id);
        }

        #[cfg(not(feature = "database"))]
        {
            eprintln!("Warning: Database feature not enabled, ignoring --save flag");
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
