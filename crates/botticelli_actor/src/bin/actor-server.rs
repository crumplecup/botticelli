//! Botticelli Actor Server - Long-running social media automation server.
//!
//! This binary runs actor servers that execute scheduled tasks for social media
//! platforms like Discord, posting content based on narratives and knowledge tables.

#[cfg(feature = "discord")]
use botticelli_actor::{
    Actor, ActorConfig, ActorExecutionTracker, DatabaseExecutionResult, ScheduleConfig,
    SkillRegistry,
};
use botticelli_actor::{ActorServerConfig, DatabaseStatePersistence};
#[cfg(feature = "discord")]
use botticelli_database::establish_connection;
#[cfg(feature = "discord")]
use botticelli_server::ActorServer;
#[cfg(feature = "discord")]
use botticelli_server::Schedule;
use clap::Parser;
#[cfg(feature = "discord")]
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(feature = "discord")]
use std::sync::Arc;
use tracing::info;
use tracing::warn;
#[cfg(feature = "discord")]
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

#[cfg(feature = "discord")]
use botticelli_actor::{DiscordActorServer, DiscordPlatform};

#[cfg(feature = "discord")]
use serenity::http::Http;

#[cfg(feature = "discord")]
use chrono::{DateTime, Utc};

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

    #[cfg(feature = "discord")]
    {
        // Set up database state persistence if DATABASE_URL is set
        let persistence = if args.database_url.is_some() || std::env::var("DATABASE_URL").is_ok() {
            info!("Database state persistence enabled");
            match DatabaseStatePersistence::new() {
                Ok(p) => {
                    info!("Created connection pool for state persistence");
                    Some(Arc::new(p))
                }
                Err(e) => {
                    warn!("Failed to create persistence: {}", e);
                    warn!("Continuing without state persistence");
                    None
                }
            }
        } else {
            warn!("DATABASE_URL not set - state persistence disabled");
            None
        };

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

        // Track actors, their schedules, last run time, and execution trackers
        let mut actors: HashMap<
            String,
            (
                Actor,
                ScheduleConfig,
                Option<DateTime<Utc>>,
                Option<ActorExecutionTracker<DatabaseStatePersistence>>,
            ),
        > = HashMap::new();

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
                let actor = Actor::builder()
                    .config(actor_config)
                    .skills(SkillRegistry::new())
                    .platform(std::sync::Arc::new(platform))
                    .build()?;

                info!(actor = %actor_instance.name, "Actor created successfully");

                // Load previous state from database if available
                let mut loaded_last_run = None;
                if let Some(ref persistence) = persistence {
                    match persistence.load_task_state(&actor_instance.name).await {
                        Ok(Some(state)) => {
                            info!(
                                actor = %actor_instance.name,
                                consecutive_failures = ?state.consecutive_failures,
                                is_paused = ?state.is_paused,
                                "Loaded previous task state from database"
                            );
                            loaded_last_run = state
                                .last_run
                                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc));
                        }
                        Ok(None) => {
                            debug!(actor = %actor_instance.name, "No previous state found");
                        }
                        Err(e) => {
                            warn!(
                                actor = %actor_instance.name,
                                error = ?e,
                                "Failed to load previous state"
                            );
                        }
                    }
                }

                // Create execution tracker if persistence is enabled
                let tracker = persistence.as_ref().map(|p| {
                    ActorExecutionTracker::new(
                        p.clone(),
                        actor_instance.name.clone(),
                        actor_instance.name.clone(),
                    )
                });

                // Store actor with schedule, last run, and tracker
                actors.insert(
                    actor_instance.name.clone(),
                    (
                        actor,
                        actor_instance.schedule.clone(),
                        loaded_last_run,
                        tracker,
                    ),
                );

                match &actor_instance.schedule {
                    ScheduleConfig::Interval { seconds } => {
                        info!(
                            actor = %actor_instance.name,
                            interval_seconds = seconds,
                            "Scheduled with interval"
                        );
                    }
                    ScheduleConfig::Immediate => {
                        info!(
                            actor = %actor_instance.name,
                            "Scheduled for immediate execution"
                        );
                    }
                    ScheduleConfig::Cron { expression } => {
                        info!(
                            actor = %actor_instance.name,
                            cron = expression,
                            "Scheduled with cron"
                        );
                    }
                    ScheduleConfig::Once { at } => {
                        info!(
                            actor = %actor_instance.name,
                            scheduled_at = at,
                            "Scheduled for one-time execution"
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
        let shutdown_flag = Arc::new(tokio::sync::Notify::new());
        let shutdown_flag_clone = shutdown_flag.clone();

        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install CTRL+C signal handler");
            shutdown_flag_clone.notify_one();
        });

        info!("Actor server starting");

        // Start the server
        server
            .start()
            .await
            .map_err(|e| format!("Failed to start server: {}", e))?;

        info!("Actor server running. Press CTRL+C to shutdown.");

        // Main execution loop
        let check_interval =
            std::time::Duration::from_secs(server_config.server.check_interval_seconds);
        let mut interval = tokio::time::interval(check_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    debug!("Checking for ready actors");

                    // Check each actor's schedule
                    for (name, (actor, schedule, last_run, tracker)) in actors.iter_mut() {
                        // Check circuit breaker if tracker available
                        if let Some(tracker) = tracker.as_ref() {
                            match tracker.should_execute().await {
                                Ok(should_run) => {
                                    if !should_run {
                                        debug!(actor = %name, "Task paused by circuit breaker, skipping");
                                        continue;
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        actor = %name,
                                        error = ?e,
                                        "Failed to check circuit breaker state, skipping"
                                    );
                                    continue;
                                }
                            }
                        }

                        let check = schedule.check(*last_run);

                        if check.should_run {
                            info!(actor = %name, "Executing scheduled actor");

                            // Start execution history if tracker available
                            let exec_id = if let Some(tracker) = tracker.as_ref() {
                                match tracker.start_execution().await {
                                    Ok(id) => {
                                        debug!(actor = %name, exec_id = id, "Started execution record");
                                        Some(id)
                                    }
                                    Err(e) => {
                                        warn!(
                                            actor = %name,
                                            error = ?e,
                                            "Failed to start execution record"
                                        );
                                        None
                                    }
                                }
                            } else {
                                None
                            };

                            // Get database connection
                            match establish_connection() {
                                Ok(mut conn) => {
                                    // Execute the actor
                                    match actor.execute(&mut conn).await {
                                        Ok(result) => {
                                            info!(
                                                actor = %name,
                                                skills_succeeded = result.succeeded.len(),
                                                skills_failed = result.failed.len(),
                                                skills_skipped = result.skipped.len(),
                                                "Actor executed successfully"
                                            );
                                            *last_run = Some(Utc::now());

                                            // Record success if tracker available
                                            if let Some(exec_id) = exec_id {
                                                if let Some(tracker) = tracker.as_ref() {
                                                    let db_result = DatabaseExecutionResult {
                                                        skills_succeeded: result.succeeded.len() as i32,
                                                        skills_failed: result.failed.len() as i32,
                                                        skills_skipped: result.skipped.len() as i32,
                                                        metadata: serde_json::json!({}),
                                                    };

                                                    if let Err(e) =
                                                        tracker.record_success(exec_id, db_result).await
                                                    {
                                                        warn!(
                                                            actor = %name,
                                                            error = ?e,
                                                            "Failed to record success"
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!(actor = %name, error = ?e, "Actor execution failed");

                                            // Record failure if tracker available
                                            if let Some(exec_id) = exec_id {
                                                if let Some(tracker) = tracker.as_ref() {
                                                    match tracker
                                                        .record_failure(exec_id, &e.to_string())
                                                        .await
                                                    {
                                                        Ok(should_pause) => {
                                                            if should_pause {
                                                                warn!(
                                                                    actor = %name,
                                                                    "Circuit breaker triggered, task paused"
                                                                );
                                                            }
                                                        }
                                                        Err(e) => {
                                                            warn!(
                                                                actor = %name,
                                                                error = ?e,
                                                                "Failed to record failure"
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(actor = %name, error = ?e, "Failed to establish database connection");

                                    // Record connection failure if tracker available
                                    if let Some(exec_id) = exec_id {
                                        if let Some(tracker) = tracker.as_ref() {
                                            if let Err(e) =
                                                tracker.record_failure(exec_id, &e.to_string()).await
                                            {
                                                warn!(
                                                    actor = %name,
                                                    error = ?e,
                                                    "Failed to record connection failure"
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(next) = check.next_run {
                                debug!(actor = %name, next_run = %next, "Next execution scheduled");
                            }
                        }
                    }
                }
                _ = shutdown_flag.notified() => {
                    info!("Shutdown signal received, stopping gracefully...");
                    break;
                }
            }
        }

        // Save final state before shutdown if persistence available
        if let Some(ref persistence) = persistence {
            info!("Saving final task state to database");
            for (name, (_, _, last_run, _)) in &actors {
                if let Some(last_run_time) = last_run {
                    match persistence.load_task_state(name).await {
                        Ok(Some(mut state)) => {
                            state.last_run = Some(last_run_time.naive_utc());
                            if let Err(e) = persistence.save_task_state(name, &state).await {
                                warn!(
                                    actor = %name,
                                    error = ?e,
                                    "Failed to save final state"
                                );
                            } else {
                                debug!(actor = %name, "Saved final state");
                            }
                        }
                        Ok(None) => {
                            debug!(actor = %name, "No state to update on shutdown");
                        }
                        Err(e) => {
                            warn!(
                                actor = %name,
                                error = ?e,
                                "Failed to load state for final save"
                            );
                        }
                    }
                }
            }
        }

        // Graceful shutdown
        server
            .stop()
            .await
            .map_err(|e| format!("Failed to stop server: {}", e))?;

        info!("Actor server stopped successfully");
        Ok(())
    }

    #[cfg(not(feature = "discord"))]
    {
        eprintln!("Discord feature not enabled. Rebuild with --features discord");
        Err("Discord feature required".into())
    }
}
