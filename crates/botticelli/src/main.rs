//! Botticelli CLI binary.
//!
//! This binary provides command-line access to Botticelli's functionality:
//! - Execute narratives from TOML files
//! - Launch TUI for content review
//! - Manage and query generated content

use clap::Parser;

mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use cli::{Cli, Commands, handle_content_command, launch_tui, run_narrative};

    // Load environment variables from .env file (if present)
    let _ = dotenvy::dotenv();

    // Parse command-line arguments
    let cli = Cli::parse();

    // Initialize tracing
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Execute the requested command
    match cli.command {
        Commands::Run {
            narrative,
            narrative_name,
            save,
            process_discord,
            state_dir,
        } => {
            #[cfg(feature = "database")]
            {
                run_narrative(
                    &narrative,
                    narrative_name.as_deref(),
                    save,
                    process_discord,
                    state_dir.as_deref(),
                )
                .await?;
            }
            #[cfg(not(feature = "database"))]
            {
                let _ = state_dir; // Suppress unused warning
                run_narrative(&narrative, narrative_name.as_deref(), save, process_discord).await?;
            }
        }

        Commands::Tui { table } => {
            launch_tui(&table).await?;
        }

        Commands::Content(content_cmd) => {
            handle_content_command(content_cmd).await?;
        }
    }

    Ok(())
}
