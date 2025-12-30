use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Test A: Verify MCP server starts, is reachable, and has correct version
#[test]
#[ignore] // Run manually with: cargo test test_mcp_server_startup -- --ignored --nocapture
fn test_mcp_server_startup() {
    println!("Testing MCP server startup and health...");

    // Start the TUI which should auto-start the MCP server
    let mut child = Command::new("just")
        .arg("chat")
        .arg("rebuild")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start just chat");

    // Wait for server to start (including compilation time on first run)
    println!("Waiting 30 seconds for MCP server to compile and start...");
    thread::sleep(Duration::from_secs(30));

    // Check health endpoint
    println!("Checking /health endpoint...");
    let health_response = reqwest::blocking::get("http://localhost:3030/health")
        .expect("Failed to connect to MCP server /health endpoint");

    assert!(
        health_response.status().is_success(),
        "Health endpoint returned non-success status: {}",
        health_response.status()
    );

    let health_json: serde_json::Value = health_response
        .json()
        .expect("Failed to parse health response as JSON");

    println!(
        "Health response: {}",
        serde_json::to_string_pretty(&health_json).unwrap()
    );
    assert_eq!(
        health_json["status"], "healthy",
        "Expected status to be 'healthy'"
    );

    // Check info endpoint for version
    println!("\nChecking /info endpoint...");
    let info_response = reqwest::blocking::get("http://localhost:3030/info")
        .expect("Failed to connect to MCP server /info endpoint");

    assert!(
        info_response.status().is_success(),
        "Info endpoint returned non-success status: {}",
        info_response.status()
    );

    let info_json: serde_json::Value = info_response
        .json()
        .expect("Failed to parse info response as JSON");

    println!(
        "Server info: {}",
        serde_json::to_string_pretty(&info_json).unwrap()
    );

    // Verify version field exists and is non-empty
    let version = info_json["version"]
        .as_str()
        .expect("Version field missing or not a string");
    assert!(!version.is_empty(), "Server version should not be empty");

    // Version should follow semver format (basic check)
    assert!(
        version.split('.').count() >= 2,
        "Server version should follow semver format: {}",
        version
    );

    // Verify server has a name
    let name = info_json["name"]
        .as_str()
        .expect("Name field missing or not a string");
    assert!(!name.is_empty(), "Server name should not be empty");

    println!("\n=== MCP Server Verification ===");
    println!("✓ Health endpoint responding");
    println!("✓ Version correct: {}", version);
    println!("✓ Server name: {}", name);

    // Clean up
    child.kill().expect("Failed to kill process");
}

/// Test B: Verify MCP client can connect to server via HTTP and list tools
#[tokio::test]
#[ignore] // Run manually with: cargo test test_mcp_client_connection -- --ignored --nocapture
async fn test_mcp_client_connection() {
    println!("Testing MCP client connection to server...");

    // First ensure server is running
    let server_running = reqwest::get("http://localhost:3030/health")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    if !server_running {
        // Start the server
        println!("Starting MCP server...");
        let _child = Command::new("just")
            .arg("mcp")
            .arg("rebuild")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start MCP server");

        // Wait for it to be ready
        println!("Waiting for server to be ready...");
        thread::sleep(Duration::from_secs(3));
    }

    // Create HTTP transport and connect
    println!("Creating HTTP transport to MCP server...");
    use botticelli_mcp_client::{HttpTransport, McpTransport};

    let mut transport = HttpTransport::new("http://localhost:3030");

    println!("Connecting to MCP server...");
    let connect_result = transport.connect().await;

    match connect_result {
        Ok(_) => {
            println!("✓ Successfully connected to MCP server via HTTP transport");

            // Try to list tools
            println!("\nListing available tools via transport...");
            let tools_result = transport.list_tools().await;

            match tools_result {
                Ok(tools) => {
                    println!("Found {} tools:", tools.len());
                    for tool in &tools {
                        println!("  - {}: {}", tool.name(), tool.description());
                    }

                    assert!(
                        !tools.is_empty(),
                        "Expected at least some tools to be available"
                    );

                    println!("\n=== MCP Client Verification ===");
                    println!("✓ HTTP transport connected successfully");
                    println!("✓ Tools listed: {} available", tools.len());
                }
                Err(e) => {
                    panic!("Failed to list tools: {:?}", e);
                }
            }
        }
        Err(e) => {
            panic!("Failed to connect MCP client to server: {:?}", e);
        }
    }
}
