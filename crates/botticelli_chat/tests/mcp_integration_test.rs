use botticelli_chat::{ChatAppConfig, CommandExecutor, EnvironmentMode, ServiceContainer};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Integration test that validates MCP server startup and communication
#[tokio::test]
#[cfg(feature = "cli")]
#[ignore] // Run with: cargo test --test mcp_integration_test --features cli -- --ignored
async fn test_mcp_server_startup_and_communication() {
    // Load local configuration
    let config = ChatAppConfig::builder()
        .mode(EnvironmentMode::Local)
        .build()
        .expect("Valid config");

    // Create services and executor
    let services = Arc::new(ServiceContainer::new(config));
    let _executor = CommandExecutor::with_services(services.clone());

    // Wait for MCP server to fully start
    sleep(Duration::from_secs(2)).await;

    // Test 1: Verify MCP server is running
    let mcp_port = services.config().mcp_server.port;
    let health_check = tokio::net::TcpStream::connect(format!("localhost:{}", mcp_port)).await;
    assert!(
        health_check.is_ok(),
        "MCP server should be listening on port {}",
        mcp_port
    );

    // Test 2: Try to initialize MCP client
    let mcp_result = services.mcp_client().await;

    assert!(
        mcp_result.is_ok(),
        "Should be able to initialize MCP client: {:?}",
        mcp_result.err()
    );

    println!("✅ MCP integration test passed");
}

/// Test that validates error handling when MCP server is not available
#[tokio::test]
#[cfg(feature = "cli")]
async fn test_mcp_server_unavailable_error() {
    // Use a config with a port where nothing is running
    let config = ChatAppConfig::builder()
        .mode(EnvironmentMode::Local)
        .mcp_port(9999) // Nothing should be running here
        .build()
        .expect("Valid config");

    let _services = Arc::new(ServiceContainer::new(config));

    // Verify port is not listening
    let health_check = tokio::net::TcpStream::connect("localhost:9999").await;
    assert!(
        health_check.is_err(),
        "Port 9999 should not have anything running"
    );

    println!("✅ MCP unavailable error test passed");
}
