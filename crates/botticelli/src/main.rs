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
    #[cfg(all(feature = "tui", feature = "server"))]
    use cli::launch_server_tui;
    #[cfg(feature = "server")]
    use cli::handle_server_command;

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
            save,
            process_discord,
        } => {
            run_narrative(&narrative, save, process_discord).await?;
        }

        Commands::Tui { table } => {
            launch_tui(&table).await?;
        }

        #[cfg(all(feature = "tui", feature = "server"))]
        Commands::TuiServer => {
            launch_server_tui().await?;
        }

        Commands::Content(content_cmd) => {
            handle_content_command(content_cmd).await?;
        }

        #[cfg(feature = "server")]
        Commands::Server(server_cmd) => {
            handle_server_command(server_cmd).await?;
        }
    }

    Ok(())
}
