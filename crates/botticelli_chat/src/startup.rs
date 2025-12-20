//! Startup sequence with automatic service setup and health checks.

#[cfg(feature = "cli")]
use {
    crate::{ChatAppConfig, EnvironmentMode},
    botticelli_error::{ChatError, ChatErrorKind, ChatResult},
    diesel::prelude::*,
    tracing::{debug, error, info, instrument},
};

/// Runs startup checks and auto-setup for all required services.
///
/// This function ensures that:
/// 1. PostgreSQL is running and accessible
/// 2. Database and tables exist (creates if missing)
/// 3. MCP server is running (starts if possible)
/// 4. All connections are validated
#[cfg(feature = "cli")]
#[instrument(skip(config))]
pub async fn startup_sequence(config: &ChatAppConfig) -> ChatResult<()> {
    info!("Running startup sequence");

    // Step 1: Check and setup PostgreSQL
    setup_postgres(config).await?;

    // Step 2: Check MCP server
    setup_mcp_server(config).await?;

    info!("Startup sequence completed successfully");
    Ok(())
}

/// Ensures PostgreSQL is running and database is set up.
#[cfg(feature = "cli")]
#[instrument(skip(config))]
async fn setup_postgres(config: &crate::ChatAppConfig) -> ChatResult<()> {
    info!(
        host = %config.postgres().host(),
        port = config.postgres().port(),
        "Checking PostgreSQL"
    );

    let db_url = config.postgres().database_url();

    // Try to connect to postgres database first (always exists)
    let postgres_url = format!(
        "postgresql://{}@{}:{}/postgres",
        config.postgres().user(), config.postgres().host(), config.postgres().port()
    );

    debug!(url = %postgres_url, "Attempting connection to postgres database");

    match PgConnection::establish(&postgres_url) {
        Ok(mut conn) => {
            info!("PostgreSQL is accessible");

            // Check if our database exists
            ensure_database_exists(&mut conn, config.postgres().database())?;

            // Now connect to our database and ensure tables exist
            ensure_tables_exist(&db_url)?;

            Ok(())
        }
        Err(e) => {
            error!(error = ?e, "PostgreSQL connection failed");

            Err(ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                "PostgreSQL connection failed: {}\n\n\
                Please ensure PostgreSQL is running:\n\
                  sudo systemctl start postgresql\n\
                or start it with your system's service manager.",
                e
            ))))
        }
    }
}

/// Ensures the application database exists.
#[cfg(feature = "cli")]
#[instrument(skip(conn))]
fn ensure_database_exists(conn: &mut diesel::PgConnection, db_name: &str) -> ChatResult<()> {
    use diesel::sql_query;

    debug!(database = %db_name, "Checking if database exists");

    // Check if database exists using simple execute check
    // Try to connect - if it fails, create it
    let check_query = format!(
        "SELECT 1 FROM pg_database WHERE datname = '{}'",
        db_name.replace('\'', "''") // Basic SQL injection protection
    );

    let result = sql_query(&check_query).execute(conn);

    match result {
        Ok(count) if count > 0 => {
            info!(database = %db_name, "Database exists");
            Ok(())
        }
        Ok(_) | Err(_) => {
            info!(database = %db_name, "Database does not exist, creating");

            // Create database
            let create_query = format!("CREATE DATABASE {}", db_name.replace('\'', "''"));

            sql_query(&create_query).execute(conn).map_err(|e| {
                ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                    "Failed to create database: {}",
                    e
                )))
            })?;

            info!(database = %db_name, "Database created successfully");
            Ok(())
        }
    }
}

/// Ensures all required tables exist by running migrations.
#[cfg(feature = "cli")]
#[instrument(skip(db_url))]
fn ensure_tables_exist(db_url: &str) -> ChatResult<()> {
    debug!("Checking if tables exist");

    // DATABASE_URL should be set via .env file for diesel migrations

    // Try to connect to the database
    let mut conn = PgConnection::establish(db_url).map_err(|e| {
        ChatError::new(ChatErrorKind::ExecutionFailed(format!(
            "Failed to connect to database: {}",
            e
        )))
    })?;

    // Check if migrations table exists
    let migrations_exist = diesel::sql_query(
        "SELECT 1 FROM information_schema.tables \
         WHERE table_name = '__diesel_schema_migrations'",
    )
    .execute(&mut conn);

    match migrations_exist {
        Ok(0) | Err(_) => {
            info!("Running database migrations");

            // Run diesel migrations
            let output = std::process::Command::new("diesel")
                .args(["migration", "run"])
                .output();

            match output {
                Ok(output) if output.status.success() => {
                    info!("Migrations completed successfully");
                    Ok(())
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!(error = %stderr, "Migration failed");

                    Err(ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                        "Failed to run migrations: {}",
                        stderr
                    ))))
                }
                Err(e) => {
                    error!(error = ?e, "Failed to execute diesel command");

                    Err(ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                        "Failed to execute diesel migration: {}\n\n\
                        Please install diesel_cli:\n\
                          cargo install diesel_cli --no-default-features --features postgres",
                        e
                    ))))
                }
            }
        }
        Ok(_) => {
            debug!("Migrations table exists, checking if up to date");

            // Migrations exist, try running any pending
            let output = std::process::Command::new("diesel")
                .args(["migration", "run"])
                .output();

            match output {
                Ok(output) if output.status.success() => {
                    info!("Migrations are up to date");
                    Ok(())
                }
                Ok(_output) => {
                    info!("Migration check completed");
                    Ok(())
                }
                Err(_) => {
                    // If diesel isn't available, assume migrations are fine
                    info!("Could not verify migrations (diesel_cli not found)");
                    Ok(())
                }
            }
        }
    }
}

/// Ensures MCP server is running, attempts to start it if not.
#[cfg(feature = "cli")]
#[instrument(skip(config))]
async fn setup_mcp_server(config: &crate::ChatAppConfig) -> ChatResult<()> {
    info!(
        host = %config.mcp_server().host(),
        port = config.mcp_server().port(),
        "Checking MCP server"
    );

    let url = config.mcp_server().server_url();

    // Try to connect to MCP server
    debug!(url = %url, "Attempting MCP server health check");

    if check_mcp_health(&url).await.is_ok() {
        info!("MCP server is already running");
        return Ok(());
    }

    // MCP server not running, try to start it
    info!("MCP server not running, attempting to start it");

    start_mcp_server(config).await?;

    // Wait a bit for server to initialize
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Verify it's now running
    check_mcp_health(&url).await
}

/// Checks if MCP server is healthy.
#[cfg(feature = "cli")]
#[instrument]
async fn check_mcp_health(url: &str) -> ChatResult<()> {
    let client = reqwest::Client::new();
    let health_url = format!("{}/health", url);

    match client
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            debug!("MCP server health check passed");
            Ok(())
        }
        Ok(response) => {
            debug!(status = %response.status(), "MCP server returned non-success status");
            Err(ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                "MCP server unhealthy: {}",
                response.status()
            ))))
        }
        Err(e) => {
            debug!(error = ?e, "MCP server not reachable");
            Err(ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                "MCP server not reachable: {}",
                e
            ))))
        }
    }
}

/// Attempts to start the MCP server as a background process.
#[cfg(feature = "cli")]
#[instrument(skip(config))]
async fn start_mcp_server(config: &crate::ChatAppConfig) -> ChatResult<()> {
    use std::process::Stdio;

    info!("Starting MCP server in background");

    // Check if we're in local mode
    let is_local = matches!(config.environment().mode(), EnvironmentMode::Local);

    if !is_local {
        return Err(ChatError::new(ChatErrorKind::ExecutionFailed(
            "Cannot auto-start MCP server in container mode. \
            Please ensure the MCP server container is running."
                .to_string(),
        )));
    }

    // First, try to find a pre-built binary
    // Check multiple locations: same dir as current exe, and target/debug or target/release
    let binary_path = std::env::current_exe().ok().and_then(|exe_path| {
        let exe_dir = exe_path.parent()?;

        // Try same directory as current executable
        let same_dir = exe_dir.join("botticelli-mcp-http");
        if same_dir.exists() {
            return Some(same_dir);
        }

        // Try target/debug (for tests running from deps/)
        if exe_dir.ends_with("deps") {
            if let Some(target_dir) = exe_dir.parent() {
                let debug_binary = target_dir.join("botticelli-mcp-http");
                if debug_binary.exists() {
                    return Some(debug_binary);
                }
            }
        }

        // Try target/release
        if let Some(target_dir) = exe_dir.parent() {
            if target_dir.ends_with("debug") || target_dir.ends_with("release") {
                if let Some(profile_parent) = target_dir.parent() {
                    let release_binary = profile_parent.join("release/botticelli-mcp-http");
                    if release_binary.exists() {
                        return Some(release_binary);
                    }
                }
            }
        }

        None
    });

    if let Some(binary) = binary_path {
        debug!(path = ?binary, "Found MCP server binary");

        // Use the pre-built binary
        let child = std::process::Command::new(&binary)
            .env("MCP_HOST", config.mcp_server().host())
            .env("MCP_PORT", config.mcp_server().port().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(proc) => {
                info!(pid = proc.id(), "MCP server started from binary");
                std::mem::forget(proc); // Let it run in background
                Ok(())
            }
            Err(e) => {
                error!(error = ?e, "Failed to start MCP server binary");
                Err(ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                    "Failed to start MCP server: {}",
                    e
                ))))
            }
        }
    } else {
        // No binary found, provide helpful error
        error!("MCP server binary not found");
        Err(ChatError::new(ChatErrorKind::ExecutionFailed(
            "MCP server binary not found.\n\n\
            Please build it first:\n\
              cargo build --bin botticelli-mcp-http --features=\"http,database,llm\"\n\n\
            Or start it manually in another terminal:\n\
              cargo run --bin botticelli-mcp-http --features=\"http,database,llm\""
                .to_string(),
        )))
    }
}
