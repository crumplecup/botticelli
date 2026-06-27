//! Bot server command handler.

use botticelli_bot::{BotConfig, BotServer};
use botticelli_database::{BotStorageTableQueryRegistry, RedbStorage};
use botticelli_error::{
    BackendError, BotticelliError, BotticelliResult, ServerError, ServerErrorKind,
};
use botticelli_interface::BotStorage;
use botticelli_models::GeminiClient;
use botticelli_narrative::{
    ContentGenerationProcessor, NarrativeExecutor, ProcessorRegistry, StorageActor,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Handle the `server` command
pub async fn handle_server_command(
    _config_path: Option<PathBuf>,
    _only_bots: Option<String>,
) -> BotticelliResult<()> {
    info!("Starting bot server");

    // Load configuration
    let config = BotConfig::from_file("bot_server.toml")?;

    // Open redb storage
    let db_path = dirs::data_dir()
        .map(|d| d.join("botticelli").join("botticelli.redb"))
        .ok_or_else(|| BackendError::new("Cannot determine data directory for redb"))?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| BotticelliError::from(BackendError::new(format!("{e}"))))?;
    }

    let storage: Arc<dyn BotStorage> = Arc::new(
        RedbStorage::open(&db_path)
            .map_err(|e| BotticelliError::from(BackendError::new(format!("{e}"))))?,
    );

    // Create Gemini client
    let client = GeminiClient::new()?;

    // Create table query registry backed by storage
    let table_registry = BotStorageTableQueryRegistry::new(Arc::clone(&storage));

    // Start storage actor with Ractor
    info!("Starting storage actor");
    let actor = StorageActor::new(Arc::clone(&storage));
    let (actor_ref, _handle) = ractor::Actor::spawn(None, actor, Arc::clone(&storage))
        .await
        .map_err(|e| {
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
    let server = BotServer::new(config, executor, storage);

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
