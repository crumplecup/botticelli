//! Integration tests for BotticelliClient transport variants.
//!
//! Covers HTTP, in-process duplex, and direct `connect_duplex` paths.
//! Each test exercises the full handshake so broken-pipe and premature-drop
//! bugs surface immediately.

use axum::{Router, body::Body, http::Request};
use botticelli_mcp::BotticelliServer;
use botticelli_mcp_client::BotticelliClient;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    tower::{StreamableHttpServerConfig, StreamableHttpService},
};
use std::sync::Arc;
use tower::ServiceBuilder;

/// Bind an HTTP MCP server on a random OS-assigned port and return that port.
///
/// The server runs for the lifetime of the test process.
#[tracing::instrument]
async fn start_http_server() -> u16 {
    let config = StreamableHttpServerConfig::default().with_stateful_mode(true);
    let http_service = StreamableHttpService::new(
        || Ok(BotticelliServer::new()),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let app = Router::new()
        .route("/health", axum::routing::get(|| async { "OK" }))
        .fallback_service(ServiceBuilder::new().service(tower::service_fn(
            move |req: Request<Body>| {
                let mut service = http_service.clone();
                async move { tower::Service::call(&mut service, req).await }
            },
        )));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind test server");
    let port = listener
        .local_addr()
        .expect("failed to get local address")
        .port();
    tokio::spawn(async move { axum::serve(listener, app).await.ok() });
    port
}

/// Extract all text content from a tool call result.
fn content_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| c.raw.as_text().map(|t| t.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── HTTP transport ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_client_connects_and_lists_tools() {
    let port = start_http_server().await;
    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = BotticelliClient::connect_http(&url)
        .await
        .expect("client should connect to local server");
    let tools = client
        .list_tools()
        .await
        .expect("list_tools should succeed");
    assert!(
        !tools.tools.is_empty(),
        "server should expose at least one tool"
    );
    let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        names.contains(&"echo"),
        "echo should be listed; got: {names:?}"
    );
}

#[tokio::test]
async fn test_client_calls_echo() {
    let port = start_http_server().await;
    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = BotticelliClient::connect_http(&url)
        .await
        .expect("client should connect to local server");
    let mut args = serde_json::Map::new();
    args.insert(
        "message".into(),
        serde_json::Value::String("hello from client test".into()),
    );
    let result = client
        .call_tool("echo", Some(args))
        .await
        .expect("echo call should succeed");
    let text = content_text(&result);
    assert!(
        text.contains("hello from client test"),
        "echo should return the message; got: {text}"
    );
}

// ── In-process transport (connect_in_process) ─────────────────────────────────

/// Verifies that `connect_in_process` completes the MCP handshake without a
/// broken-pipe error (regression for: server task dropping RunningService).
#[tokio::test]
async fn test_inproc_connects_and_lists_tools() {
    let client = BotticelliClient::connect_in_process()
        .await
        .expect("in-process client should connect without broken pipe");
    let tools = client
        .list_tools()
        .await
        .expect("list_tools should succeed over in-process transport");
    assert!(
        !tools.tools.is_empty(),
        "in-process server should expose at least one tool"
    );
    let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        names.contains(&"echo"),
        "echo should be listed over in-process transport; got: {names:?}"
    );
}

#[tokio::test]
async fn test_inproc_calls_echo() {
    let client = BotticelliClient::connect_in_process()
        .await
        .expect("in-process client should connect");
    let mut args = serde_json::Map::new();
    args.insert(
        "message".into(),
        serde_json::Value::String("hello in-process".into()),
    );
    let result = client
        .call_tool("echo", Some(args))
        .await
        .expect("echo call should succeed over in-process transport");
    let text = content_text(&result);
    assert!(
        text.contains("hello in-process"),
        "echo should return the message over in-process transport; got: {text}"
    );
}

// ── connect_duplex (caller-owned server spawn) ────────────────────────────────

/// Verifies that the caller is responsible for keeping `RunningService` alive.
/// If the spawned task drops the service without calling `waiting()`, the
/// client sees a broken pipe. This test uses the correct pattern.
#[tokio::test]
async fn test_duplex_caller_keeps_service_alive() {
    let (server_io, client_io) = tokio::io::duplex(65_536);
    tokio::spawn(async move {
        match rmcp::serve_server(BotticelliServer::new(), server_io).await {
            Ok(service) => {
                service.waiting().await.ok();
            }
            Err(e) => panic!("server startup failed: {e}"),
        }
    });
    let client = BotticelliClient::connect_duplex(client_io)
        .await
        .expect("duplex client should connect when server task holds RunningService");
    let tools = client
        .list_tools()
        .await
        .expect("list_tools should succeed over duplex transport");
    assert!(
        !tools.tools.is_empty(),
        "duplex server should expose at least one tool"
    );
}
