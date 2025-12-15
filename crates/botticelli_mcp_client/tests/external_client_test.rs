//! Integration tests for external MCP server connectivity.

use botticelli_mcp_client::{ExternalMcpClient, ExternalServerConfig};
use serde_json::json;

/// Test connecting to the official filesystem MCP server via npx.
#[tokio::test]
#[ignore] // Requires Node.js, npx, and network - run explicitly with --ignored
async fn test_connect_to_filesystem_server() {
    let config = ExternalServerConfig::builder()
        .name("filesystem".to_string())
        .command("npx".to_string())
        .args(vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
            "/tmp".to_string(),
        ])
        .build();

    let result = ExternalMcpClient::connect(config).await;
    
    assert!(
        result.is_ok(),
        "Should successfully connect to filesystem server: {:?}",
        result.err()
    );

    let client = result.unwrap();
    
    // Verify we got some tools
    let tools = client.tools();
    assert!(!tools.is_empty(), "Should discover tools from filesystem server");
    
    // Check for expected filesystem tools
    let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(
        tool_names.contains(&"read_file") || tool_names.contains(&"readFile"),
        "Filesystem server should expose read_file tool, found: {:?}",
        tool_names
    );
}

/// Test calling a tool on an external server.
#[tokio::test]
#[ignore] // Requires Node.js, npx, and network
async fn test_call_filesystem_tool() {
    use std::fs;

    // Create a test file
    let test_dir = std::env::temp_dir();
    let test_file = test_dir.join("botticelli_mcp_test.txt");
    let test_content = "Hello from Botticelli MCP client!";
    fs::write(&test_file, test_content).expect("Failed to write test file");

    // Connect to filesystem server
    let config = ExternalServerConfig::builder()
        .name("filesystem".to_string())
        .command("npx".to_string())
        .args(vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
            test_dir.to_str().unwrap().to_string(),
        ])
        .build();

    let mut client = ExternalMcpClient::connect(config)
        .await
        .expect("Should connect to filesystem server");

    // Find the read_file tool
    let tools = client.tools();
    let read_tool_name = tools
        .iter()
        .find(|t| t.name == "read_file" || t.name == "readFile")
        .map(|t| t.name.clone())
        .expect("Should have read_file tool");

    // Call the read tool
    let result = client
        .call_tool(
            &read_tool_name,
            json!({
                "path": test_file.to_str().unwrap()
            }),
        )
        .await;

    // Cleanup
    fs::remove_file(&test_file).ok();

    // Verify result
    assert!(
        result.is_ok(),
        "Should successfully read file via MCP: {:?}",
        result.err()
    );

    let response = result.unwrap();
    let response_str = serde_json::to_string(&response).unwrap();
    assert!(
        response_str.contains(test_content),
        "Response should contain our test content: {}",
        response_str
    );
}
