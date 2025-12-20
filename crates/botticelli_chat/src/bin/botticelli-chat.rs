//! Botticelli Chat Interface Binary
//!
//! Interactive chat interface for directing botticelli operations.

use botticelli_chat::{ChatAppConfig, EnvironmentMode};
use botticelli_mcp_client::{UnifiedMcpClient, register_internal_tools};
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, warn};

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
    // Load .env file if present
    let _ = dotenvy::dotenv();

    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    info!("Starting Botticelli Chat Interface");

    // Load configuration
    let config = load_config(&args)?;

    info!(
        mode = ?config.environment().mode(),
        postgres_host = %config.postgres().host(),
        mcp_host = %config.mcp_server().host(),
        "Configuration loaded"
    );

    // Run startup sequence (health checks + auto-setup)
    if !args.skip_health_checks {
        info!("Running startup sequence");
        botticelli_chat::startup_sequence(&config).await?;
    } else {
        info!("Skipping startup checks");
    }

    // Initialize MCP client
    info!("Initializing MCP client");
    let mut mcp_client = UnifiedMcpClient::builder().build();
    
    // Register internal narrative tools
    // Database feature is in botticelli_mcp_client, always pass None for now
    match register_internal_tools(mcp_client.internal_registry_mut(), "./narratives", None) {
        Ok(()) => info!("Internal narrative tools registered"),
        Err(e) => warn!(error = ?e, "Failed to register internal tools"),
    }
    
    // Connect to external MCP servers from configuration
    // TODO: Parse MCP server config from TOML to get command and args
    // For now, commenting out as we need proper config structure
    /*
    let mcp_url = config.mcp_server.server_url();
    let external_config = ExternalServerConfig::builder()
        .name("botticelli-mcp".to_string())
        .command("node".to_string())
        .args(vec!["path/to/server.js".to_string()])
        .build();
        
    match mcp_client.connect_external_server(external_config).await {
        Ok(()) => info!(url = %mcp_url, "Connected to external MCP server"),
        Err(e) => warn!(error = ?e, url = %mcp_url, "Failed to connect to external MCP server"),
    }
    */
    
    // List all available tools
    let available_tools = mcp_client.list_all_tools();
    info!(
        tool_count = available_tools.len(),
        tools = ?available_tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
        "MCP tools loaded"
    );

    info!("All dependencies ready");

    // Show configuration summary
    println!("\nBotticelli Chat Interface");
    println!("========================");
    println!("\nConfiguration loaded:");
    println!("  Mode:       {:?}", config.environment().mode());
    println!(
        "  Postgres:   {}:{}",
        config.postgres().host(), config.postgres().port()
    );
    println!(
        "  MCP Server: {}:{}",
        config.mcp_server().host(), config.mcp_server().port()
    );
    println!("\nStarting interactive chat...");
    println!("Press Ctrl+C to exit\n");

    // Small delay for user to read
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    info!("Setting up conversation components");

    // Wrap MCP client in Arc<RwLock> for shared access
    let mcp_client = std::sync::Arc::new(tokio::sync::RwLock::new(mcp_client));
    
    // Create tool call handler
    let tool_handler = botticelli_chat::ToolCallHandler::new(mcp_client.clone());
    let tool_handler = std::sync::Arc::new(tokio::sync::RwLock::new(tool_handler));
    
    // Create conversation loop
    let _conversation_loop = botticelli_chat::ConversationLoop::new(tool_handler.clone());

    info!("Starting TUI interface");

    // Create service container with configuration
    let _services = std::sync::Arc::new(botticelli_chat::ServiceContainer::new(config));

    // Create and run new TUI app
    // TODO: Pass mcp_client and conversation components to TUI
    let mut tui = botticelli_tui::Tui::new()?;

    // Run the app
    let result = tui.run().await;

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
                return Err(
                    format!("Invalid mode: {}. Use 'local' or 'container'", mode_str).into(),
                )
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
    if let Some(user) = &args.postgres_user {
        builder = builder.postgres_user(user);
    }
    if let Some(password) = &args.postgres_password {
        builder = builder.postgres_password(password);
    }
    if let Some(database) = &args.postgres_database {
        builder = builder.postgres_database(database);
    }

    // Apply MCP overrides
    if let Some(host) = &args.mcp_host {
        builder = builder.mcp_host(host);
    }
    if let Some(port) = args.mcp_port {
        builder = builder.mcp_port(port);
    }

    let config = builder.build()?;

    Ok(config)
}
