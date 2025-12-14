use botticelli_actor::{WorkflowConfigBuilder, WorkflowExecutor};
use botticelli_database::create_pool_from_url;
use clap::Parser;
use std::process;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "demo-workflow")]
#[command(about = "Execute Botticelli demo workflow with validation", long_about = None)]
struct Args {
    /// Database URL.
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// Delay between stages in milliseconds.
    #[arg(long, default_value = "2000")]
    stage_delay_ms: u64,

    /// Run in test mode (skip actual execution).
    #[arg(long)]
    test_mode: bool,

    /// Verbose logging.
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let log_level = if args.verbose { "debug" } else { "info" };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}={}", env!("CARGO_PKG_NAME"), log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Botticelli demo workflow");
    warn!("Note: This demo assumes PostgreSQL and MCP server are already running");
    warn!("Run 'just chat-local' first to start all required services");

    let db_pool = match create_pool_from_url(&args.database_url) {
        Ok(pool) => {
            info!("Database connection pool created");
            Some(pool)
        }
        Err(e) => {
            if args.test_mode {
                info!("Test mode - continuing without database");
                None
            } else {
                error!(error = %e, "Failed to connect to database");
                eprintln!("Error: Failed to connect to database: {}", e);
                eprintln!("Hint: Run 'just chat-local' to start all services");
                process::exit(1);
            }
        }
    };

    let config = WorkflowConfigBuilder::default()
        .db_pool(db_pool)
        .mcp_endpoint("http://localhost:3000".to_string())
        .stage_delay_ms(args.stage_delay_ms)
        .test_mode(args.test_mode)
        .build()
        .expect("Valid configuration");

    let mut executor = WorkflowExecutor::new(config);

    match executor.execute().await {
        Ok(summary) => {
            info!("Workflow completed successfully");
            summary.print();
        }
        Err(e) => {
            error!(error = %e, "Workflow failed");
            eprintln!("Error: Workflow failed: {}", e);
            process::exit(1);
        }
    }
}
