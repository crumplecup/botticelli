//! Botticelli Actor Server - Long-running social media automation server.
//!
//! This binary runs actor servers that execute scheduled tasks for social media
//! platforms like Discord, posting content based on narratives and knowledge tables.

use botticelli_actor::ActorServerConfig;
#[cfg(feature = "discord")]
use botticelli_actor::{
    Actor, ActorConfig, ActorExecutionTracker, DatabaseExecutionResult, DatabaseStatePersistence,
    NarrativeExecutionSkill, ScheduleConfig, SkillRegistry,
};
#[cfg(feature = "discord")]
use botticelli_database::create_pool;
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
    // Load environment variables from .env file (if present)
    let _ = dotenvy::dotenv();

    // Initialize observability (tracing + metrics + optional OTLP export)
    #[cfg(feature = "observability")]
    {
        let config = botticelli::ObservabilityConfig::new("botticelli-actor-server")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_metrics(false); // Disable metrics for now (traces only)
        botticelli::init_observability_with_config(config)?;
        info!(
            "Observability initialized (OTEL_EXPORTER={:?})",
            std::env::var("OTEL_EXPORTER").unwrap_or_else(|_| "stdout".to_string())
        );
    }

    // Fallback to basic tracing if observability feature not enabled
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
        // Type alias for actor tracking
        type ActorEntry = (
            Actor,
            ScheduleConfig,
            Option<DateTime<Utc>>,
            Option<ActorExecutionTracker<DatabaseStatePersistence>>,
        );

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

        // Create database connection pool for actor execution
        info!("Creating database connection pool for actor execution");
        let db_pool = create_pool()?;
        info!("Database connection pool created");

        // Initialize server with state file path
        let state_path = PathBuf::from(".actor_server_state.json");
        let mut server = DiscordActorServer::new(http.clone(), state_path);

        // Track actors, their schedules, last run time, and execution trackers
        let mut actors: HashMap<String, ActorEntry> = HashMap::new();

        // Initialize metrics for the server (if enabled)
        #[cfg(feature = "metrics")]
        let metrics = {
            let m = Arc::new(ServerMetrics::new());
            info!("Server metrics initialized");
            info!("Metrics enabled - exporting via OTLP");
            m
        };

        #[cfg(not(feature = "metrics"))]
        info!("Metrics disabled");

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

            // Create platform (Discord if channel_id provided, NoOp otherwise)
            let platform: Arc<dyn botticelli_actor::Platform> =
                if let Some(channel_id) = &actor_instance.channel_id {
                    info!(
                        actor = %actor_instance.name,
                        channel_id = %channel_id,
                        "Creating Discord platform for actor"
                    );
                    {
                        let _ = discord_token; // Token validation happens elsewhere
                        Arc::new(DiscordPlatform::new(channel_id)?)
                    }
                } else {
                    info!(
                        actor = %actor_instance.name,
                        "No channel_id specified, using NoOpPlatform (actor will not post)"
                    );
                    Arc::new(botticelli_actor::NoOpPlatform::new())
                };

            // Create skill registry and register narrative execution skill
            let mut registry = SkillRegistry::new();
            registry.register(Arc::new(NarrativeExecutionSkill::new()));

            // Build actor with platform and skills
            let actor = Actor::builder()
                .config(actor_config)
                .skills(registry)
                .platform(platform)
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

        // Test metric: Record server startup (if metrics enabled)
        #[cfg(feature = "metrics")]
        {
            info!("Recording test startup metric");
            metrics.bots.record_execution("test_startup", 0.0);
            info!("Test startup metric recorded");
        }

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

                            // Execute the actor with database pool
                            let start_time = std::time::Instant::now();
                            match actor.execute(&db_pool).await {
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

                                    // Record metrics (if enabled)
                                    #[cfg(feature = "metrics")]
                                    metrics.bots.record_execution(name, duration);

                                    // Record success if tracker available
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

                                    // Record failure metric (if enabled)
                                    #[cfg(feature = "metrics")]
                                    metrics.bots.record_failure(name);

                                    // Record failure if tracker available
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
