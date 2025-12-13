//! Botticelli Chat Interface Binary
//!
//! Interactive chat interface for directing botticelli operations.

use botticelli_chat::{ChatAppConfig, EnvironmentMode};
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Parser)]
#[command(name = "botticelli-chat")]
#[command(about = "Interactive chat interface for Botticelli", long_about = None)]
#[command(version)]
struct Args {
    /// Environment mode: local or container
    #[arg(short, long, env = "BOTTICELLI__ENVIRONMENT__MODE")]
    mode: Option<String>,

    /// Path to configuration file
    #[arg(short, long, env = "BOTTICELLI__CONFIG")]
    config: Option<PathBuf>,

    /// PostgreSQL host
    #[arg(long, env = "BOTTICELLI__POSTGRES__HOST")]
    postgres_host: Option<String>,

    /// PostgreSQL port
    #[arg(long, env = "BOTTICELLI__POSTGRES__PORT")]
    postgres_port: Option<u16>,

    /// PostgreSQL user
    #[arg(long, env = "BOTTICELLI__POSTGRES__USER")]
    postgres_user: Option<String>,

    /// PostgreSQL password
    #[arg(long, env = "BOTTICELLI__POSTGRES__PASSWORD")]
    postgres_password: Option<String>,

    /// PostgreSQL database name
    #[arg(long, env = "BOTTICELLI__POSTGRES__DATABASE")]
    postgres_database: Option<String>,

    /// MCP server host
    #[arg(long, env = "BOTTICELLI__MCP_SERVER__HOST")]
    mcp_host: Option<String>,

    /// MCP server port
    #[arg(long, env = "BOTTICELLI__MCP_SERVER__PORT")]
    mcp_port: Option<u16>,

    /// Skip health checks
    #[arg(long)]
    skip_health_checks: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    info!("Starting Botticelli Chat Interface");

    // Load configuration
    let config = load_config(&args)?;

    info!(
        mode = ?config.environment.mode,
        postgres_host = %config.postgres.host,
        mcp_host = %config.mcp_server.host,
        "Configuration loaded"
    );

    // Run startup sequence (health checks + auto-setup)
    if !args.skip_health_checks {
        info!("Running startup sequence");
        botticelli_chat::startup_sequence(&config).await?;
    } else {
        info!("Skipping startup checks");
    }

    // Initialize services
    let _db_url = config.postgres.database_url();
    let _mcp_url = config.mcp_server.server_url();

    info!("All dependencies ready");
    
    // Show configuration summary
    println!("\nBotticelli Chat Interface");
    println!("========================");
    println!("\nConfiguration loaded:");
    println!("  Mode:       {:?}", config.environment.mode);
    println!("  Postgres:   {}:{}", config.postgres.host, config.postgres.port);
    println!("  MCP Server: {}:{}", config.mcp_server.host, config.mcp_server.port);
    println!("\nStarting interactive chat...");
    println!("Press Ctrl+C to exit\n");

    // Small delay for user to read
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    info!("Starting TUI interface");

    // Create service container with configuration
    let services = std::sync::Arc::new(botticelli_chat::ServiceContainer::new(config));

    // Setup terminal for TUI
    let mut terminal = botticelli_chat::tui::setup_terminal()
        .map_err(|e| format!("Failed to setup terminal: {}", e))?;

    // Create TUI interface with services
    let mut tui = botticelli_chat::tui::TuiInterface::with_services(services);

    // Run the chat loop
    let result = tui.run(&mut terminal).await;

    // Restore terminal
    botticelli_chat::tui::restore_terminal(terminal)
        .map_err(|e| format!("Failed to restore terminal: {}", e))?;

    match result {
        Ok(()) => {
            info!("Chat interface exited normally");
            Ok(())
        }
        Err(e) => {
            eprintln!("Chat interface error: {}", e);
            Err(format!("Chat error: {}", e).into())
        }
    }
}

fn init_logging(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let log_level = if args.verbose { "debug" } else { "info" };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .init();

    Ok(())
}

fn load_config(args: &Args) -> Result<ChatAppConfig, Box<dyn std::error::Error>> {
    let mut builder = ChatAppConfig::builder();

    // Apply config file if provided
    if let Some(path) = &args.config {
        builder = builder.config_path(path);
    }

    // Apply mode if provided
    if let Some(mode_str) = &args.mode {
        let mode = match mode_str.to_lowercase().as_str() {
            "local" => EnvironmentMode::Local,
            "container" => EnvironmentMode::Container,
            _ => {
                return Err(format!("Invalid mode: {}. Use 'local' or 'container'", mode_str).into())
            }
        };
        builder = builder.mode(mode);
    }

    // Apply postgres overrides
    if let Some(host) = &args.postgres_host {
        builder = builder.postgres_host(host);
    }
    if let Some(port) = args.postgres_port {
        builder = builder.postgres_port(port);
    }

    // Apply MCP overrides
    if let Some(host) = &args.mcp_host {
        builder = builder.mcp_host(host);
    }
    if let Some(port) = args.mcp_port {
        builder = builder.mcp_port(port);
    }

    let mut config = builder.build()?;

    // Apply additional postgres overrides not in builder
    if let Some(user) = &args.postgres_user {
        config.postgres.user = user.clone();
    }
    if let Some(password) = &args.postgres_password {
        config.postgres.password = password.clone();
    }
    if let Some(database) = &args.postgres_database {
        config.postgres.database = database.clone();
    }

    Ok(config)
}
