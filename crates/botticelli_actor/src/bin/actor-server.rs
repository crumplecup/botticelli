//! Botticelli Actor Server - Long-running social media automation server.
//!
//! This binary runs actor servers that execute scheduled tasks for social media
//! platforms like Discord, posting content based on narratives and knowledge tables.

use botticelli_actor::ActorServerConfig;
#[cfg(feature = "discord")]
use botticelli_actor::{
    Actor, ActorConfig, ActorExecutionTracker, BotStorageStatePersistence, DatabaseExecutionResult,
    NarrativeExecutionSkill, ScheduleConfig, SkillRegistry,
};
#[cfg(feature = "discord")]
use botticelli_database::RedbStorage;
#[cfg(feature = "discord")]
use botticelli_interface::BotStorage;
#[cfg(feature = "discord")]
use botticelli_server::ActorServer;
#[cfg(feature = "discord")]
use botticelli_server::Schedule;
#[cfg(all(feature = "discord", feature = "metrics"))]
use botticelli_server::ServerMetrics;
use clap::Parser;
#[cfg(feature = "discord")]
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(feature = "discord")]
use std::sync::Arc;
use tracing::info;
#[cfg(feature = "discord")]
use tracing::{debug, error, warn};
#[cfg(not(feature = "observability"))]
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

    /// Path to redb database file (defaults to platform data dir)
    #[arg(long, env = "BOTTICELLI_DB")]
    db_path: Option<PathBuf>,

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
    // Load environment variables from .env file (if present)
    let _ = dotenvy::dotenv();

    // Initialize observability (tracing + metrics + optional OTLP export)
    #[cfg(feature = "observability")]
    {
        let config = botticelli_core::ObservabilityConfig::new("botticelli-actor-server")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_metrics(false);
        botticelli_core::init_observability_with_config(config)?;
        info!(
            "Observability initialized (OTEL_EXPORTER={:?})",
            std::env::var("OTEL_EXPORTER").unwrap_or_else(|_| "stdout".to_string())
        );
    }

    #[cfg(not(feature = "observability"))]
    {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .init();
    }

    let args = Args::parse();
    info!("Starting Botticelli Actor Server");
    info!(config_file = ?args.config, "Loading configuration");

    let server_config = ActorServerConfig::from_file(&args.config)?;
    info!(
        actors = server_config.actors.len(),
        check_interval = server_config.server.check_interval_seconds,
        "Configuration loaded"
    );

    if args.dry_run {
        info!("DRY RUN MODE - No actions will be executed");
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
        // Determine redb database path
        let db_path = args
            .db_path
            .or_else(|| dirs::data_dir().map(|d| d.join("botticelli").join("botticelli.redb")))
            .ok_or("Cannot determine database path")?;

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        info!(db_path = %db_path.display(), "Opening redb storage");
        let storage: Arc<dyn BotStorage> = Arc::new(RedbStorage::open(&db_path)?);
        let persistence = Arc::new(BotStorageStatePersistence::new(Arc::clone(&storage)));

        // Type alias for actor tracking
        type ActorEntry = (
            Actor,
            ScheduleConfig,
            Option<DateTime<Utc>>,
            Option<ActorExecutionTracker<BotStorageStatePersistence>>,
        );

        // Initialize Discord server
        let discord_token = args
            .discord_token
            .or_else(|| std::env::var("DISCORD_TOKEN").ok())
            .ok_or("DISCORD_TOKEN not provided")?;

        let http = Arc::new(Http::new(&discord_token));

        let state_path = PathBuf::from(".actor_server_state.json");
        let mut server = DiscordActorServer::new(http.clone(), state_path);

        let mut actors: HashMap<String, ActorEntry> = HashMap::new();

        #[cfg(feature = "metrics")]
        let metrics = {
            let m = Arc::new(ServerMetrics::new());
            info!("Server metrics initialized");
            info!("Metrics enabled - exporting via OTLP");
            m
        };

        #[cfg(not(feature = "metrics"))]
        info!("Metrics disabled");

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

            let actor_config = ActorConfig::from_file(&actor_instance.config_file)?;

            let platform: Arc<dyn botticelli_actor::Platform> =
                if let Some(channel_id) = &actor_instance.channel_id {
                    info!(
                        actor = %actor_instance.name,
                        channel_id = %channel_id,
                        "Creating Discord platform for actor"
                    );
                    {
                        let _ = discord_token.as_str();
                        Arc::new(DiscordPlatform::new(channel_id)?)
                    }
                } else {
                    info!(
                        actor = %actor_instance.name,
                        "No channel_id specified, using NoOpPlatform (actor will not post)"
                    );
                    Arc::new(botticelli_actor::NoOpPlatform::new())
                };

            let mut registry = SkillRegistry::new();
            registry.register(Arc::new(NarrativeExecutionSkill::new()));

            let actor = Actor::builder()
                .config(actor_config)
                .skills(registry)
                .platform(platform)
                .build()?;

            info!(actor = %actor_instance.name, "Actor created successfully");

            // Load previous state from storage if available
            let mut loaded_last_run: Option<DateTime<Utc>> = None;
            match persistence.load_task_state(&actor_instance.name).await {
                Ok(Some(state)) => {
                    info!(
                        actor = %actor_instance.name,
                        consecutive_failures = state.consecutive_failures,
                        is_paused = state.is_paused,
                        "Loaded previous task state"
                    );
                    loaded_last_run = state.last_run;
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

            let tracker = ActorExecutionTracker::new(
                Arc::clone(&persistence),
                actor_instance.name.clone(),
                actor_instance.name.clone(),
            );

            actors.insert(
                actor_instance.name.clone(),
                (
                    actor,
                    actor_instance.schedule.clone(),
                    loaded_last_run,
                    Some(tracker),
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
            }
        }

        let shutdown_flag = Arc::new(tokio::sync::Notify::new());
        let shutdown_flag_clone = shutdown_flag.clone();

        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install CTRL+C signal handler");
            shutdown_flag_clone.notify_one();
        });

        info!("Actor server starting");

        server
            .start()
            .await
            .map_err(|e| format!("Failed to start server: {}", e))?;

        info!("Actor server running. Press CTRL+C to shutdown.");

        #[cfg(feature = "metrics")]
        {
            info!("Recording test startup metric");
            metrics.bots.record_execution("test_startup", 0.0);
            info!("Test startup metric recorded");
        }

        let check_interval =
            std::time::Duration::from_secs(server_config.server.check_interval_seconds);
        let mut interval = tokio::time::interval(check_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    debug!("Checking for ready actors");

                    for (name, (actor, schedule, last_run, tracker)) in actors.iter_mut() {
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

                            let exec_id = if let Some(tracker) = tracker.as_ref() {
                                match tracker.start_execution().await {
                                    Ok(id) => {
                                        debug!(actor = %name, exec_id = %id, "Started execution record");
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

                            let start_time = std::time::Instant::now();
                            match actor.execute(&storage).await {
                                Ok(result) => {
                                    let duration = start_time.elapsed().as_secs_f64();
                                    info!(
                                        actor = %name,
                                        skills_succeeded = result.succeeded.len(),
                                        skills_failed = result.failed.len(),
                                        skills_skipped = result.skipped.len(),
                                        duration_secs = duration,
                                        "Actor executed successfully"
                                    );
                                    *last_run = Some(Utc::now());

                                    #[cfg(feature = "metrics")]
                                    metrics.bots.record_execution(name, duration);

                                    if let Some(exec_id) = exec_id
                                        && let Some(tracker) = tracker.as_ref() {
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
                                Err(e) => {
                                    error!(actor = %name, error = ?e, "Actor execution failed");

                                    #[cfg(feature = "metrics")]
                                    metrics.bots.record_failure(name);

                                    if let Some(exec_id) = exec_id
                                        && let Some(tracker) = tracker.as_ref() {
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

        // Save final state before shutdown
        info!("Saving final task state");
        for (name, (_, _, last_run, _)) in &actors {
            if let Some(last_run_time) = last_run {
                match persistence.load_task_state(name).await {
                    Ok(Some(mut state)) => {
                        state.last_run = Some(*last_run_time);
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
