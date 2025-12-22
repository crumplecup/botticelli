//! Botticelli Chat Interface Binary
//!
//! Interactive chat interface for directing botticelli operations.

use botticelli_chat::{ChatAppConfig, EnvironmentMode};
#[cfg(feature = "database")]
use botticelli_database::create_pool_from_url;
use botticelli_interface::ToolCalling;
use botticelli_mcp_client::McpHost;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, debug};

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
#[tracing::instrument(name = "main")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    tracing::info!("Starting Botticelli Chat Interface");

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

    // Initialize database pool if database feature is enabled
    #[cfg(feature = "database")]
    let _db_pool = {
        info!("Creating database connection pool");
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.postgres().user(),
            config.postgres().password(),
            config.postgres().host(),
            config.postgres().port(),
            config.postgres().database()
        );
        match create_pool_from_url(&database_url) {
            Ok(pool) => {
                info!("Database connection pool created");
                Some(pool)
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to create database connection pool");
                tracing::warn!("Continuing without database - some tools will be unavailable");
                None
            }
        }
    };

    // Initialize MCP client and connect to subprocess server
    tracing::info!("Initializing MCP client");
    let mcp_host = initialize_mcp_client().await?;
    
    // Log available tools
    let available_tools = mcp_host.list_all_tools();
    tracing::info!(
        tool_count = available_tools.len(),
        tools = ?available_tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
        "MCP tools loaded from HTTP server"
    );

    info!("All dependencies ready");

    // Show configuration summary
    println!("\nBotticelli Chat Interface");
    println!("========================");
    println!("\nLogs: botticelli-chat.log");
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

    info!("Starting TUI interface");

    // Create service container with configuration
    let _services = std::sync::Arc::new(botticelli_chat::ServiceContainer::new(config));

    // Initialize LLM backend with fallback
    let chat_host = match initialize_llm_backend().await {
        Ok(llm_backend) => {
            info!("LLM backend initialized successfully");
            // Create MCP host and chat host with LLM
            let mcp_host = Arc::new(tokio::sync::Mutex::new(
                McpHost::builder()
                    .build()
            ));
            let chat_host = botticelli_chat::McpChatHost::new(mcp_host, llm_backend);
            Some(Arc::new(std::sync::Mutex::new(chat_host)) as Arc<std::sync::Mutex<dyn botticelli_interface::ChatHost>>)
        }
        Err(e) => {
            warn!(error = ?e, "Failed to initialize LLM backend, continuing without it");
            warn!("Set GEMINI_API_KEY or ANTHROPIC_API_KEY in .env file to enable LLM features");
            None
        }
    };

    // Create TUI with optional chat host
    let mut tui = botticelli_tui::Tui::with_llm(chat_host)?;

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

#[tracing::instrument(skip_all, name = "initialize_mcp_client")]
async fn initialize_mcp_client() -> Result<McpHost, Box<dyn std::error::Error>> {
    tracing::debug!("Building MCP host");
    
    // Get narratives directory from environment or use default
    let narratives_dir = std::env::var("NARRATIVES_DIR")
        .unwrap_or_else(|_| "./narratives".to_string());
    
    // Initialize database pool if DATABASE_URL is set
    #[cfg(feature = "database")]
    let db_pool = {
        use botticelli_database::create_pool;
        match std::env::var("DATABASE_URL") {
            Ok(_url) => {
                tracing::info!("Initializing database connection pool");
                match create_pool() {
                    Ok(pool) => {
                        tracing::info!("Database pool created successfully");
                        Some(pool)
                    }
                    Err(e) => {
                        tracing::warn!(error = ?e, "Failed to create database pool, continuing without database tools");
                        None
                    }
                }
            }
            Err(_) => {
                tracing::info!("DATABASE_URL not set, skipping database tools");
                None
            }
        }
    };
    
    // Create MCP host
    let mut mcp_host = botticelli_mcp_client::McpHost::builder().build();
    
    // Register internal narrative and elicitation tools
    tracing::info!(narratives_dir = %narratives_dir, "Registering internal tools");
    botticelli_mcp_client::register_internal_tools(
        mcp_host.internal_registry_mut(),
        &narratives_dir,
        #[cfg(feature = "database")]
        db_pool,
    )?;
    
    // List all available tools
    let tools = mcp_host.list_all_tools();
    tracing::info!(tool_count = tools.len(), "Total tools available");
    for tool in &tools {
        tracing::debug!(tool_name = %tool.name(), "Available tool");
    }
    tracing::info!("MCP host initialized");
    Ok(mcp_host)
}

#[tracing::instrument(skip_all, name = "fetch_tools_from_http_server")]
async fn _fetch_tools_from_http_server() -> Result<Vec<botticelli_core::ToolDefinition>, Box<dyn std::error::Error>> {
    let server_url = std::env::var("MCP_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    
    tracing::debug!(server_url = %server_url, "Fetching tools from MCP server");
    
    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    
    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&request_body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    tracing::debug!(response = ?response, "Received response from MCP server");
    
    // Parse tools from response
    let tools_array = response
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .ok_or("Invalid response format")?;
    
    tracing::debug!(tools_count = tools_array.len(), "Found tools in response");
    
    let tools: Vec<botticelli_core::ToolDefinition> = tools_array
        .iter()
        .filter_map(|t| {
            let name = t.get("name")?.as_str()?.to_string();
            let description = t.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("No description")
                .to_string();
            let input_schema = t.get("inputSchema")?.clone();
            
            Some(botticelli_core::ToolDefinition::new(name, description, input_schema))
        })
        .collect();
    
    tracing::info!(tool_count = tools.len(), "Fetched tools from server");
    Ok(tools)
}

#[tracing::instrument(skip_all, name = "init_logging")]
fn init_logging(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let log_level = if args.verbose { "debug" } else { "info" };

    // Write logs to file to avoid interfering with TUI
    let log_file = std::fs::File::create("botticelli-chat.log")?;
    
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with_writer(std::sync::Arc::new(log_file))
        .with_ansi(false) // Disable ANSI colors in log file
        .init();

    Ok(())
}

#[tracing::instrument(skip_all, name = "load_config")]
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

/// Initialize LLM backend with fallback support.
///
/// Tries to create providers in priority order based on available API keys.
/// Priority: Gemini (default, has tool calling) -> Anthropic (fallback).
/// Note: Groq doesn't support tool calling yet.
async fn initialize_llm_backend() -> Result<Arc<dyn ToolCalling>, Box<dyn std::error::Error>> {
    tracing::debug!("Initializing LLM backend");

    // Try Gemini first (has tool calling support, generous free tier)
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        if !api_key.is_empty() {
            tracing::debug!("Found GEMINI_API_KEY, creating Gemini provider");
            match botticelli_models::GeminiClient::new() {
                Ok(client) => {
                    tracing::info!("Using Gemini as LLM backend");
                    return Ok(Arc::new(client) as Arc<dyn ToolCalling>);
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "Failed to create Gemini client");
                }
            }
        } else {
            tracing::debug!("GEMINI_API_KEY is empty, skipping");
        }
    }

    // Try Anthropic (has tool calling, no free tier)
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        if !api_key.is_empty() {
            tracing::debug!("Found ANTHROPIC_API_KEY, creating Anthropic provider");
            let client = botticelli_models::AnthropicClient::new(&api_key, "claude-3-5-sonnet-20241022");
            tracing::info!("Using Anthropic as LLM backend");
            return Ok(Arc::new(client) as Arc<dyn ToolCalling>);
        } else {
            tracing::debug!("ANTHROPIC_API_KEY is empty, skipping");
        }
    }

    Err("No LLM provider API keys found in environment. Set GEMINI_API_KEY or ANTHROPIC_API_KEY in .env file. Note: Groq doesn't support tool calling yet.".into())
}
