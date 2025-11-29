//! Bot server command handler.

use botticelli_bot::{BotConfig, BotServer};
use botticelli_database::{DatabaseTableQueryRegistry, TableQueryExecutor, create_pool};
use botticelli_error::{BotticelliResult, ServerError, ServerErrorKind};
use botticelli_models::GeminiClient;
use botticelli_narrative::{ContentGenerationProcessor, NarrativeExecutor, ProcessorRegistry, StorageActor};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::info;

/// Handle the `server` command
pub async fn handle_server_command(
    _config_path: Option<PathBuf>,
    _only_bots: Option<String>,
) -> BotticelliResult<()> {
    info!("Starting bot server");

    // Load configuration
    let config = BotConfig::from_file("bot_server.toml")?;

    // Establish database connection pool
    let pool = create_pool()?;

    // Create Gemini client
    let client = GeminiClient::new()?;

    // Create database connection for table queries
    let table_conn = botticelli::establish_connection()?;
    let table_executor = TableQueryExecutor::new(Arc::new(Mutex::new(table_conn)));
    let table_registry = DatabaseTableQueryRegistry::new(table_executor);

    // Start storage actor with Ractor
    info!("Starting storage actor");
    let actor = StorageActor::new(pool.clone());
    let (actor_ref, _handle) =
        ractor::Actor::spawn(None, actor, pool.clone()).await.map_err(|e| {
            ServerError::new(ServerErrorKind::ServerStartFailed(format!(
                "Failed to spawn storage actor: {}",
                e
            )))
        })?;
    info!("Storage actor started");

    // Create content generation processor with storage actor
    let processor = ContentGenerationProcessor::new(actor_ref);
    let mut registry = ProcessorRegistry::new();
    registry.register(Box::new(processor));

    // Create narrative executor with processors and table registry
    let mut executor = NarrativeExecutor::with_processors(client, registry);
    executor = executor.with_table_registry(Box::new(table_registry));

    // Create and start server
    let server = BotServer::new(config, executor, pool);

    info!("Starting bot server with configured intervals");

    // Start server with metrics on port 9090
    server
        .start(Some(9090))
        .await
        .map_err(|e| ServerError::new(ServerErrorKind::ServerStartFailed(e.to_string())))?;

    info!("Bot server running. Metrics available at http://localhost:9090/metrics");
    info!("Press Ctrl+C to stop.");

    // Keep server running
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| ServerError::new(ServerErrorKind::ServerStopFailed(e.to_string())))?;

    info!("Bot server shutdown complete");

    Ok(())
}
