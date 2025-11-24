//! Botticelli Actor Server - Long-running social media automation server.
//!
//! This binary runs actor servers that execute scheduled tasks for social media
//! platforms like Discord, posting content based on narratives and knowledge tables.

use botticelli_actor::{
    Actor, ActorConfig, ActorServerConfig, DatabaseStatePersistence, ScheduleConfig,
    SkillRegistry,
};
use botticelli_server::{ActorServer, StatePersistence};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[cfg(feature = "discord")]
use botticelli_actor::{DiscordActorServer, DiscordPlatform};

#[cfg(feature = "discord")]
use serenity::http::Http;

/// Command-line arguments for the actor server.
#[derive(Parser, Debug)]
#[command(name = "actor-server")]
#[command(about = "Botticelli Actor Server - Social media automation")]
#[command(version)]
struct Args {
    /// Path to server configuration file
    #[arg(short, long, default_value = "actor_server.toml")]
    config: PathBuf,

    /// Database URL for state persistence
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Discord bot token
    #[arg(long, env = "DISCORD_TOKEN")]
    #[cfg(feature = "discord")]
    discord_token: Option<String>,

    /// Dry run mode (don't actually execute actors)
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    info!("Starting Botticelli Actor Server");
    info!(config_file = ?args.config, "Loading configuration");

    // Load server configuration
    let server_config = ActorServerConfig::from_file(&args.config)?;
    info!(
        actors = server_config.actors.len(),
        check_interval = server_config.server.check_interval_seconds,
        "Configuration loaded"
    );

    if args.dry_run {
        info!("DRY RUN MODE - No actions will be executed");
        // Just validate configuration and exit
        for actor_instance in &server_config.actors {
            info!(
                actor = %actor_instance.name,
                config = %actor_instance.config_file,
                enabled = actor_instance.enabled,
                "Actor configuration validated"
            );
        }
        info!("Configuration validation complete");
        return Ok(());
    }

    // Set up database state persistence if DATABASE_URL is set
    if args.database_url.is_some() || std::env::var("DATABASE_URL").is_ok() {
        info!("Database state persistence enabled");
        let persistence = DatabaseStatePersistence::new();

        // Attempt to load previous state
        match persistence.load_state().await {
            Ok(Some(state)) => {
                info!(
                    task_id = %state.task_id,
                    actor = %state.actor_name,
                    "Loaded previous server state from database"
                );
            }
            Ok(None) => {
                info!("No previous server state found in database");
            }
            Err(e) => {
                warn!("Failed to load previous state: {}", e);
            }
        }
    } else {
        warn!("DATABASE_URL not set - state persistence disabled");
    }

    #[cfg(feature = "discord")]
    {
        // Initialize Discord server
        let discord_token = args
            .discord_token
            .or_else(|| std::env::var("DISCORD_TOKEN").ok())
            .ok_or("DISCORD_TOKEN not provided")?;

        // Create Discord HTTP client
        let http = Arc::new(Http::new(&discord_token));

        // Initialize server with state file path
        let state_path = PathBuf::from(".actor_server_state.json");
        let mut server = DiscordActorServer::new(http.clone(), state_path);

        // Load and register actors from configuration
        for actor_instance in &server_config.actors {
            if !actor_instance.enabled {
                info!(actor = %actor_instance.name, "Actor disabled, skipping");
                continue;
            }

            info!(
                actor = %actor_instance.name,
                config_file = %actor_instance.config_file,
                "Loading actor"
            );

            // Load actor configuration
            let actor_config = ActorConfig::from_file(&actor_instance.config_file)?;

            // Create Discord platform if channel_id is provided
            if let Some(channel_id) = &actor_instance.channel_id {
                let platform = DiscordPlatform::new(&discord_token, channel_id)?;

                // Build actor with platform
                let _actor = Actor::builder()
                    .config(actor_config)
                    .skills(SkillRegistry::new())
                    .platform(std::sync::Arc::new(platform))
                    .build()?;

                info!(actor = %actor_instance.name, "Actor created successfully");

                // For now, we'll just validate the actor was created
                // In Phase 4, we'll add scheduling support
                match &actor_instance.schedule {
                    ScheduleConfig::Interval { seconds } => {
                        info!(
                            actor = %actor_instance.name,
                            interval_seconds = seconds,
                            "Scheduled with interval (Phase 4 not yet implemented)"
                        );
                    }
                    ScheduleConfig::Immediate => {
                        info!(
                            actor = %actor_instance.name,
                            "Scheduled for immediate execution (Phase 4 not yet implemented)"
                        );
                    }
                    ScheduleConfig::Cron { expression } => {
                        info!(
                            actor = %actor_instance.name,
                            cron = expression,
                            "Scheduled with cron (Phase 4 not yet implemented)"
                        );
                    }
                    ScheduleConfig::Once { at } => {
                        info!(
                            actor = %actor_instance.name,
                            scheduled_at = at,
                            "Scheduled for one-time execution (Phase 4 not yet implemented)"
                        );
                    }
                }
            } else {
                warn!(
                    actor = %actor_instance.name,
                    "No channel_id specified, actor will not post"
                );
            }
        }

        // Set up graceful shutdown signal handler
        let shutdown = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install CTRL+C signal handler");
        };

        info!("Actor server starting");

        // Start the server
        server
            .start()
            .await
            .map_err(|e| format!("Failed to start server: {}", e))?;

        info!("Actor server running. Press CTRL+C to shutdown.");

        // Wait for shutdown signal
        shutdown.await;

        info!("Shutdown signal received, stopping gracefully...");

        // Graceful shutdown
        server
            .stop()
            .await
            .map_err(|e| format!("Failed to stop server: {}", e))?;

        info!("Actor server stopped successfully");
    }

    #[cfg(not(feature = "discord"))]
    {
        eprintln!("Discord feature not enabled. Rebuild with --features discord");
        return Err("Discord feature required".into());
    }

    Ok(())
}
