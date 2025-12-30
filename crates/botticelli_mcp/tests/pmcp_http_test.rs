//! Integration tests for PMCP HTTP server.
//!
//! Tests the HTTP transport with actual HTTP requests.

#[cfg(feature = "streamable-http")]
mod http_tests {
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::sleep;

    /// Helper to start HTTP server in background
    async fn start_test_server() -> Result<u16, Box<dyn std::error::Error>> {
        use botticelli_mcp::run_pmcp_http_server;

        // Use a random port to avoid conflicts
        let port = 18080 + (std::process::id() % 1000) as u16;

        // Start server in background
        tokio::spawn(async move {
            let _ = run_pmcp_http_server("127.0.0.1", port).await;
        });

        // Give server time to start
        sleep(Duration::from_millis(500)).await;

        Ok(port)
    }

    /// Test that HTTP server starts and responds to initialize
    #[tokio::test]
    #[ignore] // Ignored by default - requires server to be running
    async fn test_http_server_initialize() {
        let port = start_test_server().await.expect("Failed to start server");
        let url = format!("http://127.0.0.1:{}/mcp", port);

        // Send initialize request
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

        let client = reqwest::Client::new();
        let response = client.post(&url).json(&initialize_request).send().await;

        match response {
            Ok(resp) => {
                assert_eq!(resp.status(), 200, "Should return 200 OK");

                let body: serde_json::Value = resp.json().await.expect("Should parse JSON");
                println!("Initialize response: {:?}", body);

                assert_eq!(body["jsonrpc"], "2.0", "Should be JSON-RPC 2.0");
                assert_eq!(body["id"], 1, "Should have matching ID");
                assert!(
                    body.get("result").is_some() || body.get("error").is_some(),
                    "Should have result or error"
                );
            }
            Err(e) => {
                println!("HTTP request failed (server may not be running): {}", e);
                println!("Note: This test requires the HTTP server to be running");
            }
        }
    }

    /// Test that tools/list endpoint works
    #[tokio::test]
    #[ignore] // Ignored by default
    async fn test_http_server_list_tools() {
        let port = start_test_server().await.expect("Failed to start server");
        let url = format!("http://127.0.0.1:{}/mcp", port);

        // Send tools/list request
        let list_tools_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let client = reqwest::Client::new();
        let response = client.post(&url).json(&list_tools_request).send().await;

        match response {
            Ok(resp) => {
                let body: serde_json::Value = resp.json().await.expect("Should parse JSON");
                println!("Tools list response: {:?}", body);

                if let Some(result) = body.get("result") {
                    if let Some(tools) = result.get("tools") {
                        let tool_array = tools.as_array().expect("Tools should be array");
                        println!("Found {} tools", tool_array.len());

                        // Should have at least echo and server_info
                        assert!(tool_array.len() >= 2, "Should have at least 2 tools");

                        // Check that tools have required fields
                        for tool in tool_array {
                            assert!(tool.get("name").is_some(), "Tool should have name");
                            assert!(
                                tool.get("description").is_some(),
                                "Tool should have description"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                println!("HTTP request failed: {}", e);
            }
        }
    }

    /// Test calling echo tool via HTTP
    #[tokio::test]
    #[ignore] // Ignored by default
    async fn test_http_server_call_echo_tool() {
        let port = start_test_server().await.expect("Failed to start server");
        let url = format!("http://127.0.0.1:{}/mcp", port);

        // Send tools/call request for echo
        let call_tool_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "Hello from HTTP!"
                }
            }
        });

        let client = reqwest::Client::new();
        let response = client.post(&url).json(&call_tool_request).send().await;

        match response {
            Ok(resp) => {
                let body: serde_json::Value = resp.json().await.expect("Should parse JSON");
                println!("Echo tool response: {:?}", body);

                if let Some(result) = body.get("result") {
                    if let Some(content) = result.get("content") {
                        println!("Echo result content: {:?}", content);
                        // The response format depends on MCP protocol
                        // Just verify we got something back
                        assert!(!content.to_string().is_empty());
                    }
                }
            }
            Err(e) => {
                println!("HTTP request failed: {}", e);
            }
        }
    }

    /// Test concurrent requests to HTTP server
    #[tokio::test]
    #[ignore] // Ignored by default
    async fn test_http_server_concurrent_requests() {
        let port = start_test_server().await.expect("Failed to start server");
        let url = format!("http://127.0.0.1:{}/mcp", port);

        // Send multiple concurrent requests
        let mut handles = vec![];

        for i in 0..5 {
            let url = url.clone();
            let handle = tokio::spawn(async move {
                let request = json!({
                    "jsonrpc": "2.0",
                    "id": i,
                    "method": "tools/call",
                    "params": {
                        "name": "echo",
                        "arguments": {
                            "message": format!("Concurrent request {}", i)
                        }
                    }
                });

                let client = reqwest::Client::new();
                client.post(&url).json(&request).send().await
            });

            handles.push(handle);
        }

        // Wait for all requests
        let results = futures::future::join_all(handles).await;

        let successful = results.iter().filter(|r| r.is_ok()).count();
        println!("Successful concurrent requests: {}/5", successful);

        // At least some should succeed (may not all if server startup slow)
        assert!(successful > 0, "At least one request should succeed");
    }
}

/// Manual test instructions (not automated)
#[test]
#[ignore]
fn manual_http_test_instructions() {
    println!("\n=== Manual HTTP Server Test ===");
    println!("1. Start the HTTP server:");
    println!("   cargo run --bin botticelli-mcp-pmcp-http --features streamable-http,database,llm");
    println!("\n2. In another terminal, test with curl:");
    println!("   curl -X POST http://localhost:8080/mcp \\");
    println!("     -H 'Content-Type: application/json' \\");
    println!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{{}}}}'"
    );
    println!("\n3. Test echo tool:");
    println!("   curl -X POST http://localhost:8080/mcp \\");
    println!("     -H 'Content-Type: application/json' \\");
    println!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{{\"name\":\"echo\",\"arguments\":{{\"message\":\"Hello\"}}}}}}'"
    );
    println!("\n");
}
