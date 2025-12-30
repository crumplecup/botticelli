use std::time::Duration;
use tokio::time::timeout;
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_http_server_connection() {
    // Setup tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .init();

    tracing::info!("Starting HTTP connection test");

    // Start MCP HTTP server
    let server_handle = tokio::spawn(async {
        tracing::info!("Starting MCP HTTP server on port 3030");
        // This would start the actual server
        // For now, just simulate it
        tokio::time::sleep(Duration::from_secs(60)).await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Try to connect
    tracing::info!("Attempting to connect to http://localhost:3030");
    let client = reqwest::Client::new();

    match timeout(
        Duration::from_secs(5),
        client.get("http://localhost:3030/health").send(),
    )
    .await
    {
        Ok(Ok(response)) => {
            tracing::info!(
                "✓ HTTP server connected successfully: {}",
                response.status()
            );
            assert!(response.status().is_success());
        }
        Ok(Err(e)) => {
            tracing::error!("✗ Connection failed: {}", e);
            panic!("Failed to connect to HTTP server");
        }
        Err(_) => {
            tracing::error!("✗ Connection timeout");
            panic!("Connection timeout");
        }
    }

    server_handle.abort();
    tracing::info!("Test complete");
}
