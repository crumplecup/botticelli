//! Demo mode binary that exercises chat interface with automated prompts.

use botticelli_actor::{create_mcp_tools_demo, ChatDemoExecutor, DemoExecutor};
use botticelli_chat::{ChatAppConfig, ChatError, ChatErrorKind, ChatResult, ServiceContainer};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, instrument};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("Starting Botticelli Demo Mode");

    run_demo().await.map_err(|e| {
        error!(error = ?e, "Demo failed");
        Box::new(e) as Box<dyn std::error::Error>
    })
}

#[instrument]
async fn run_demo() -> ChatResult<()> {
    let config = ChatAppConfig::load(None).map_err(|e| {
        ChatError::new(ChatErrorKind::IoError(format!("Config load failed: {}", e)))
    })?;
    info!(mode = ?config.environment.mode, "Configuration loaded");

    let _services = Arc::new(ServiceContainer::new(config));
    info!("Services initialized");

    let (input_tx, mut input_rx) = mpsc::unbounded_channel();
    let (output_tx, output_rx) = mpsc::unbounded_channel();

    let mut executor = ChatDemoExecutor::new(input_tx, output_rx);

    let scenario = create_mcp_tools_demo().map_err(|e| {
        ChatError::new(ChatErrorKind::InvalidInput(format!(
            "Failed to create demo scenario: {}",
            e
        )))
    })?;

    info!(scenario = %scenario.name(), "Created demo scenario");

    let chat_task = tokio::spawn(async move {
        while let Some(prompt) = input_rx.recv().await {
            info!(prompt = %prompt, "Received prompt in chat");
            let response = format!("Processed: {}", prompt);
            if output_tx.send(response).is_err() {
                error!("Failed to send response");
                break;
            }
        }
    });

    let results = executor.execute_scenario(&scenario).await.map_err(|e| {
        ChatError::new(ChatErrorKind::InvalidState(format!(
            "Demo execution failed: {}",
            e
        )))
    })?;

    chat_task.abort();

    info!(responses = results.len(), "Demo completed successfully");
    for (i, response) in results.iter().enumerate() {
        info!(response_num = i + 1, response_preview = %response.chars().take(100).collect::<String>(), "Response received");
    }

    Ok(())
}
