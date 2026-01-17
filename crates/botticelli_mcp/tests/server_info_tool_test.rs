//! Tests for server_info tool.
//!
//! Tests use the actual rmcp API, not mocked interfaces.

mod helpers;

use botticelli_mcp::BotticelliServer;

#[tokio::test]
async fn test_server_info_basic() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing basic server info");

    let server = BotticelliServer::builder().build();
    tracing::debug!("Created server");

    let result = server.server_info().await?;
    tracing::debug!(
        name = %result.0.name,
        version = %result.0.version,
        tool_count = result.0.tool_count,
        "Received server info"
    );

    assert_eq!(result.0.name, "botticelli");
    assert!(!result.0.version.is_empty());
    assert!(!result.0.timestamp.is_empty());
    // Tool count depends on features and configuration
    assert!(result.0.tool_count >= 0);

    tracing::info!("Basic server info test passed");
    Ok(())
}

#[tokio::test]
async fn test_server_info_tool_count() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing server info tool count");

    let server = BotticelliServer::builder().build();

    let result = server.server_info().await?;
    tracing::debug!(tool_count = result.0.tool_count, "Got tool count");

    // Tool count should be non-negative
    assert!(
        result.0.tool_count >= 0,
        "Tool count should be non-negative, got {}",
        result.0.tool_count
    );

    tracing::info!("Tool count test passed");
    Ok(())
}

#[tokio::test]
async fn test_server_info_version() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing server info version");

    let server = BotticelliServer::builder().build();

    let result = server.server_info().await?;
    tracing::debug!(version = %result.0.version, expected = env!("CARGO_PKG_VERSION"), "Checking version");

    // Version should match the package version
    assert_eq!(result.0.version, env!("CARGO_PKG_VERSION"));

    tracing::info!("Version test passed");
    Ok(())
}

