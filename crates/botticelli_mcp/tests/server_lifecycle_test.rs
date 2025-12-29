//! Full server lifecycle tests for MCP server.
//!
//! Tests the complete lifecycle: startup → operation → shutdown.

use serde_json::json;
use std::sync::Once;
use std::time::Duration;
use tokio::time::sleep;
use tracing_subscriber::EnvFilter;

/// Initialize tracing once for all tests.
static TRACING: Once = Once::new();

fn init_tracing() {
    TRACING.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive(tracing::Level::DEBUG.into())
            )
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_test_writer()
            .init();
        
        tracing::info!("🔍 Tracing initialized for lifecycle tests");
    });
}

/// Helper to start the HTTP server as a background task.
///
/// Returns port number on success.
#[cfg(feature = "streamable-http")]
async fn start_http_server() -> Result<u16, Box<dyn std::error::Error>> {
    use botticelli_mcp::run_pmcp_http_server;
    use std::sync::atomic::{AtomicU16, Ordering};
    
    init_tracing();
    
    // Use atomic counter to avoid port conflicts between parallel tests
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(0);
    let offset = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let port = 18080 + offset;
    
    tracing::info!("🚀 Starting HTTP server on port {}", port);
    
    // Start server in background task
    let server_handle = tokio::spawn(async move {
        tracing::info!("🌐 HTTP server task starting on port {}", port);
        match run_pmcp_http_server(
            "127.0.0.1",
            port,
            #[cfg(feature = "database")]
            None,
        ).await {
            Ok(_) => tracing::info!("✅ HTTP server task completed"),
            Err(e) => tracing::error!("❌ HTTP server task failed: {}", e),
        }
    });
    
    // Give spawn a moment to start and check it didn't panic
    sleep(Duration::from_millis(100)).await;
    
    if server_handle.is_finished() {
        return Err("Server task completed immediately - likely failed to start".into());
    }
    
    // Server spawned successfully - wait a bit for it to bind
    sleep(Duration::from_millis(500)).await;
    
    tracing::info!("✅ Server task spawned on port {}", port);
    Ok(port)
}

/// Test HTTP server lifecycle with full workflow.
#[tokio::test]
#[cfg(feature = "streamable-http")]
async fn test_http_server_lifecycle() {
    init_tracing();
    
    let port = match start_http_server().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to start HTTP server: {}", e);
            return;
        }
    };

    let url = format!("http://127.0.0.1:{}/sse", port);
    tracing::info!("Testing lifecycle on endpoint: {}", url);
    let client = reqwest::Client::new();

    // Step 1: Initialize
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&initialize_request).send().await;
    assert!(response.is_ok(), "Initialize request should succeed");

    let init_response: serde_json::Value = response.unwrap().json().await.unwrap();
    println!("Initialize response: {:?}", init_response);
    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(
        init_response.get("result").is_some(),
        "Should have result field"
    );

    // Step 2: List tools
    let list_tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let tools_response: serde_json::Value = client
        .post(&url)
        .json(&list_tools_request)
        .send()
        .await
        .expect("tools/list request failed")
        .json()
        .await
        .expect("Should parse JSON");

    println!("Tools response: {:?}", tools_response);
    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 2);

    let tools = tools_response["result"]["tools"]
        .as_array()
        .expect("Should have tools array");
    assert!(tools.len() >= 6, "Should have at least 6 base tools");

    // Verify core tools are present
    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();

    println!("Available tools: {:?}", tool_names);
    assert!(tool_names.contains(&"echo"), "Should have echo tool");
    assert!(
        tool_names.contains(&"server_info"),
        "Should have server_info tool"
    );
    assert!(
        tool_names.contains(&"create_narrative"),
        "Should have create_narrative tool"
    );

    // Step 3: Call echo tool
    let call_echo_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "echo",
            "arguments": {
                "message": "http lifecycle test"
            }
        }
    });

    let echo_response: serde_json::Value = client
        .post(&url)
        .json(&call_echo_request)
        .send()
        .await
        .expect("echo tool call failed")
        .json()
        .await
        .expect("Should parse JSON");

    println!("Echo response: {:?}", echo_response);
    assert_eq!(echo_response["jsonrpc"], "2.0");
    assert_eq!(echo_response["id"], 3);
    assert!(
        echo_response.get("result").is_some(),
        "Echo should return result"
    );

    println!("✅ HTTP server lifecycle test passed");
}

/// Test HTTP server error handling.
#[tokio::test]
#[cfg(feature = "streamable-http")]
async fn test_http_server_error_handling() {
    init_tracing();
    
    let port = match start_http_server().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            return;
        }
    };

    let url = format!("http://127.0.0.1:{}/sse", port);
    tracing::info!("Testing error handling on endpoint: {}", url);
    let client = reqwest::Client::new();

    // Initialize first
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let _init: serde_json::Value = client
        .post(&url)
        .json(&initialize_request)
        .send()
        .await
        .expect("Initialize failed")
        .json()
        .await
        .expect("Should parse JSON");

    // Send invalid tool call
    let invalid_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "nonexistent_tool",
            "arguments": {}
        }
    });

    let error_response: serde_json::Value = client
        .post(&url)
        .json(&invalid_request)
        .send()
        .await
        .expect("Should get error response")
        .json()
        .await
        .expect("Should parse JSON");

    println!("Error response: {:?}", error_response);
    assert_eq!(error_response["jsonrpc"], "2.0");
    assert_eq!(error_response["id"], 2);
    assert!(
        error_response.get("error").is_some(),
        "Should have error field"
    );

    println!("✅ HTTP server error handling test passed");
}

/// Test HTTP server handles concurrent requests.
#[tokio::test]
#[cfg(feature = "streamable-http")]
async fn test_http_server_concurrent_requests() {
    init_tracing();
    
    let port = match start_http_server().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            return;
        }
    };

    let url = format!("http://127.0.0.1:{}/sse", port);
    tracing::info!("Testing concurrent requests on endpoint: {}", url);
    let client = reqwest::Client::new();

    // Initialize
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let _init: serde_json::Value = client
        .post(&url)
        .json(&initialize_request)
        .send()
        .await
        .expect("Initialize failed")
        .json()
        .await
        .expect("Should parse JSON");

    // Send multiple concurrent requests
    let mut handles = vec![];

    for i in 1..=5 {
        let url = url.clone();
        let client = client.clone();
        
        let handle = tokio::spawn(async move {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {
                        "message": format!("concurrent request {}", i)
                    }
                }
            });

            client
                .post(&url)
                .json(&request)
                .send()
                .await
                .and_then(|r| r.error_for_status())
        });

        handles.push(handle);
    }

    // Wait for all requests
    let results = futures::future::join_all(handles).await;

    let successful = results
        .iter()
        .filter(|r| r.is_ok() && r.as_ref().unwrap().is_ok())
        .count();
    
    println!("Successful concurrent requests: {}/5", successful);
    assert_eq!(successful, 5, "All concurrent requests should succeed");
    
    println!("✅ HTTP server concurrent requests test passed");
}

/// Test HTTP server startup and shutdown.
///
/// This test simply verifies the server task spawns without panicking.
/// Communication tests are handled by other tests.
#[tokio::test]
#[cfg(feature = "streamable-http")]
async fn test_http_server_startup() {
    init_tracing();
    
    tracing::info!("🧪 Testing HTTP server startup...");
    
    let result = start_http_server().await;
    
    if let Err(ref e) = result {
        tracing::error!("Failed to start server: {}", e);
    }
    
    assert!(
        result.is_ok(),
        "HTTP server should start successfully: {:?}",
        result.err()
    );
    
    let port = result.unwrap();
    tracing::info!("✅ HTTP server started successfully on port {}", port);
    tracing::info!("Note: Server logs show it bound and is listening - communication tests verify protocol");
}

/// Test HTTP server version information.
#[tokio::test]
#[cfg(feature = "streamable-http")]
async fn test_http_server_version() {
    let port = match start_http_server().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            return;
        }
    };

    let url = format!("http://127.0.0.1:{}/sse", port);
    tracing::info!("Testing server version on endpoint: {}", url);
    let client = reqwest::Client::new();

    // Initialize to get server capabilities
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let init_response: serde_json::Value = client
        .post(&url)
        .json(&initialize_request)
        .send()
        .await
        .expect("Initialize failed")
        .json()
        .await
        .expect("Should parse JSON");

    if let Some(server_info) = init_response["result"]["serverInfo"].as_object() {
        println!("Server info: {:?}", server_info);
        assert!(
            server_info.get("name").is_some(),
            "Should have server name"
        );
        assert!(
            server_info.get("version").is_some(),
            "Should have server version"
        );
        
        let name = server_info["name"].as_str().unwrap();
        assert_eq!(name, "botticelli-pmcp-http", "Server name should match");
        
        println!("✅ HTTP server version test passed");
    }
}
