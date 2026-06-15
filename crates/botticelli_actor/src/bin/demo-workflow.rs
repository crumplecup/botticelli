use botticelli_actor::{WorkflowConfigBuilder, WorkflowExecutor};
use botticelli_database::RedbStorage;
use botticelli_interface::BotStorage;
use clap::Parser;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "demo-workflow")]
#[command(about = "Execute Botticelli demo workflow with validation", long_about = None)]
struct Args {
    /// Path to redb database file.
    #[arg(long, env = "BOTTICELLI_DB")]
    db_path: Option<PathBuf>,

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
    warn!("Note: This demo assumes MCP server is already running");
    warn!("Run 'just chat-local' first to start all required services");

    let db_path = args
        .db_path
        .or_else(|| dirs::data_dir().map(|d| d.join("botticelli").join("botticelli.redb")))
        .unwrap_or_else(|| PathBuf::from("botticelli.redb"));

    std::fs::create_dir_all(db_path.parent().unwrap_or(std::path::Path::new("."))).ok();

    let storage: Option<Arc<dyn BotStorage>> = match RedbStorage::open(&db_path) {
        Ok(s) => {
            info!("Opened redb storage");
            Some(Arc::new(s))
        }
        Err(e) => {
            if args.test_mode {
                info!("Test mode - continuing without storage");
                None
            } else {
                error!(error = %e, "Failed to open redb storage");
                eprintln!("Error: Failed to open storage: {}", e);
                process::exit(1);
            }
        }
    };

    let config = WorkflowConfigBuilder::default()
        .storage(storage)
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
