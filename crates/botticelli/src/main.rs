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
    #[cfg(feature = "gemini")]
    use botticelli_core::BudgetConfig;
    #[cfg(feature = "bots")]
    use cli::handle_server_command;
    #[cfg(feature = "gemini")]
    use cli::{
        Cli, Commands, ExecutionOptions, NarrativeSource, handle_content_command, launch_tui,
        run_narrative,
    };
    #[cfg(not(feature = "gemini"))]
    use cli::{Cli, Commands, handle_content_command, launch_tui, run_narrative};

    // Load environment variables from .env file (if present)
    let _ = dotenvy::dotenv();

    // Parse command-line arguments
    let cli = Cli::parse();

    // Initialize observability
    #[cfg(feature = "observability")]
    {
        botticelli::init_observability()?;
    }

    #[cfg(not(feature = "observability"))]
    {
        let log_level = if cli.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        };

        tracing_subscriber::fmt()
            .with_max_level(log_level)
            .with_target(false)
            .init();
    }

    // Execute the requested command
    match cli.command {
        Commands::Run {
            narrative,
            narrative_name,
            save,
            #[cfg(feature = "discord")]
            process_discord,
            #[cfg(all(feature = "gemini", feature = "database"))]
            state_dir,
            rpm_multiplier,
            tpm_multiplier,
            rpd_multiplier,
        } => {
            #[cfg(feature = "gemini")]
            {
                // Build narrative source
                let source = NarrativeSource::new(narrative, narrative_name);

                // Build execution options
                #[cfg(feature = "database")]
                let options = {
                    let builder = ExecutionOptions::builder().save(save);
                    #[cfg(feature = "discord")]
                    let builder = builder.process_discord(process_discord);
                    builder.state_dir(state_dir).build()
                };

                #[cfg(not(feature = "database"))]
                let options = {
                    let builder = ExecutionOptions::builder().save(save);
                    #[cfg(feature = "discord")]
                    let builder = builder.process_discord(process_discord);
                    builder.build()
                };

                // Build budget overrides if any multiplier is provided
                let budget_overrides = if rpm_multiplier.is_some()
                    || tpm_multiplier.is_some()
                    || rpd_multiplier.is_some()
                {
                    let mut builder = BudgetConfig::builder();
                    if let Some(rpm) = rpm_multiplier {
                        builder = builder.rpm_multiplier(rpm);
                    }
                    if let Some(tpm) = tpm_multiplier {
                        builder = builder.tpm_multiplier(tpm);
                    }
                    if let Some(rpd) = rpd_multiplier {
                        builder = builder.rpd_multiplier(rpd);
                    }
                    Some(builder.build())
                } else {
                    None
                };

                run_narrative(&source, &options, budget_overrides.as_ref()).await?;
            }

            #[cfg(not(feature = "gemini"))]
            {
                let _ = (
                    narrative,
                    narrative_name,
                    save,
                    #[cfg(feature = "discord")]
                    process_discord,
                    rpm_multiplier,
                    tpm_multiplier,
                    rpd_multiplier,
                );
                run_narrative().await?;
            }
        }

        Commands::Tui { table } => {
            launch_tui(&table).await?;
        }

        Commands::Content(content_cmd) => {
            handle_content_command(content_cmd).await?;
        }

        #[cfg(feature = "bots")]
        Commands::Server { config, only } => {
            handle_server_command(config, only).await?;
        }
    }

    // Graceful shutdown of observability
    #[cfg(feature = "observability")]
    botticelli::shutdown_observability();

    Ok(())
}
