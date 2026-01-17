//! Tests for echo tool.
//!
//! Tests use the actual rmcp API, not mocked interfaces.

mod helpers;

use botticelli_mcp::{BotticelliServer, EchoParams};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_echo_basic_message() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing basic echo functionality");

    let server = BotticelliServer::builder().build();
    tracing::debug!("Created BotticelliServer");

    let params = EchoParams {
        message: "Hello, MCP!".to_string(),
    };

    let result = server.echo(Parameters(params)).await?;
    tracing::debug!(echo = %result.0.echo, timestamp = %result.0.timestamp, "Received echo response");

    assert_eq!(result.0.echo, "Hello, MCP!");
    assert!(!result.0.timestamp.is_empty());

    tracing::info!("Basic echo test passed");
    Ok(())
}

#[tokio::test]
async fn test_echo_empty_message() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing echo with empty message");

    let server = BotticelliServer::builder().build();
    let params = EchoParams {
        message: String::new(),
    };

    let result = server.echo(Parameters(params)).await?;
    tracing::debug!(echo_len = result.0.echo.len(), "Received empty echo response");

    assert_eq!(result.0.echo, "");

    tracing::info!("Empty echo test passed");
    Ok(())
}

#[tokio::test]
async fn test_echo_unicode_message() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing echo with unicode");

    let server = BotticelliServer::builder().build();
    let params = EchoParams {
        message: "Hello 世界 🌍".to_string(),
    };

    let result = server.echo(Parameters(params)).await?;
    tracing::debug!(echo = %result.0.echo, "Received unicode echo response");

    assert_eq!(result.0.echo, "Hello 世界 🌍");

    tracing::info!("Unicode echo test passed");
    Ok(())
}

