//! Tests for server_info tool.
//!
//! Tests use the actual rmcp API, not mocked interfaces.

use botticelli_mcp::BotticelliServer;

#[tokio::test]
async fn test_server_info_basic() {
    let server = BotticelliServer::builder().build();

    let result = server
        .server_info()
        .await
        .expect("Server info should succeed");

    assert_eq!(result.0.name, "botticelli");
    assert!(!result.0.version.is_empty());
    assert!(!result.0.timestamp.is_empty());
    assert!(result.0.tool_count >= 2); // At least echo and server_info
}

#[tokio::test]
async fn test_server_info_tool_count() {
    let server = BotticelliServer::builder().build();

    let result = server
        .server_info()
        .await
        .expect("Server info should succeed");

    // We should have at least our two tools
    assert!(
        result.0.tool_count >= 2,
        "Expected at least 2 tools, got {}",
        result.0.tool_count
    );
}

#[tokio::test]
async fn test_server_info_version() {
    let server = BotticelliServer::builder().build();

    let result = server
        .server_info()
        .await
        .expect("Server info should succeed");

    // Version should match the package version
    assert_eq!(result.0.version, env!("CARGO_PKG_VERSION"));
}
