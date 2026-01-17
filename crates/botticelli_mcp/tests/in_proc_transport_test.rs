//! Tests for in-process MCP transport.
//!
//! **OBSOLETE**: These tests use the old pmcp protocol and InProcTransport API
//! which have been replaced by rmcp. Marked as ignored pending removal or rewrite.

use botticelli_mcp::InProcTransport;
use pmcp::Server;

/// Test that we can create paired transports without panicking.
///
/// **OBSOLETE**: InProcTransport::pair() removed in rmcp migration.
#[ignore = "Uses obsolete pmcp protocol - marked for removal"]
#[tokio::test]
async fn test_in_proc_transport_pair_creation() {
    let _ = tracing_subscriber::fmt::try_init();

    let (_client_transport, _server_transport) = InProcTransport::pair();

    // If we get here without panicking, the test passes
}

/// Test that we can spawn a server with the transport.
///
/// **OBSOLETE**: pmcp::Server and InProcTransport::spawn_server() removed in rmcp migration.
#[ignore = "Uses obsolete pmcp protocol - marked for removal"]
#[tokio::test]
async fn test_in_proc_transport_server_spawn() {
    let _ = tracing_subscriber::fmt::try_init();

    // Build a minimal server
    let server = Server::builder()
        .name("test-server")
        .version("0.1.0")
        .build()
        .expect("Failed to build server");

    let (_client_transport, server_transport) = InProcTransport::pair();

    // Test spawning server
    let handle = InProcTransport::spawn_server(server, server_transport);

    // Give server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Abort and verify cleanup works
    handle.abort();
}

/// Test that InProcTransport implements required traits.
///
/// **OBSOLETE**: InProcTransport removed in rmcp migration.
#[ignore = "Uses obsolete InProcTransport API - marked for removal"]
#[test]
fn test_in_proc_transport_traits() {
    // This is a compile-time test
    // If this compiles, InProcTransport implements the required traits

    fn _assert_send<T: Send>() {}
    fn _assert_sync<T: Sync>() {}
    fn _assert_debug<T: std::fmt::Debug>() {}

    _assert_send::<InProcTransport>();
    _assert_debug::<InProcTransport>();
    // Note: InProcTransport is intentionally not Sync due to interior mutability
}
