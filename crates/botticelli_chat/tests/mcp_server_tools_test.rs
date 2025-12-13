
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Test helper to start MCP server
struct McpServerGuard {
    process: Child,
}

impl McpServerGuard {
    fn start() -> Self {
        // Pre-compile the binary
        let status = Command::new("cargo")
            .args([
                "build",
                "--bin",
                "botticelli-mcp-http",
                "-p",
                "botticelli_mcp",
                "--features",
                "http",
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .expect("Failed to compile MCP server");

        assert!(status.success(), "MCP server compilation failed");

        // Run the compiled binary
        let mut process = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "botticelli-mcp-http",
                "-p",
                "botticelli_mcp",
                "--features",
                "http",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start MCP server");

        // Wait for server to start by checking stderr
        let stderr = process.stderr.take().expect("Failed to capture stderr");
        let reader = BufReader::new(stderr);

        let start = Instant::now();
        let timeout = Duration::from_secs(10);

        for line in reader.lines() {
            if start.elapsed() > timeout {
                panic!("MCP server didn't start within timeout");
            }

            if let Ok(line) = line {
                eprintln!("MCP: {}", line);
                if line.contains("Starting HTTP server") {
                    break;
                }
            }
        }

        // Give it a bit more time to bind
        thread::sleep(Duration::from_secs(1));

        Self { process }
    }
}

impl Drop for McpServerGuard {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[tokio::test]
#[ignore = "Requires MCP server with http,database,llm features"]
async fn test_mcp_server_lists_tools() {
    let _server = McpServerGuard::start();

    // Give server time to fully initialize
    thread::sleep(Duration::from_secs(1));

    // Test against localhost MCP server
    let mcp_host = "localhost";
    let mcp_port = 3000;

    let client = reqwest::Client::new();
    let url = format!("http://{}:{}/tools/list", mcp_host, mcp_port);

    let response = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to MCP server");

    assert!(
        response.status().is_success(),
        "MCP server returned error: {}",
        response.status()
    );

    let tools: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse tools response");

    // Verify tools array exists
    let tools_array = tools
        .as_object()
        .expect("Response should be object")
        .get("tools")
        .expect("Response should have 'tools' field")
        .as_array()
        .expect("'tools' should be array");

    // Find create_narrative tool
    let has_create_narrative = tools_array.iter().any(|tool| {
        tool.as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            == Some("create_narrative")
    });

    assert!(
        has_create_narrative,
        "MCP server should expose 'create_narrative' tool. Available tools: {:?}",
        tools_array
            .iter()
            .filter_map(|t| t.get("name"))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
#[ignore = "Requires MCP server with http,database,llm features"]
async fn test_mcp_server_create_narrative_tool_structure() {
    let _server = McpServerGuard::start();
    thread::sleep(Duration::from_secs(1));

    // Test against localhost MCP server
    let mcp_host = "localhost";
    let mcp_port = 3000;

    let client = reqwest::Client::new();
    let url = format!("http://{}:{}/tools/list", mcp_host, mcp_port);

    let response = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to MCP server");

    let tools: serde_json::Value = response.json().await.unwrap();
    let tools_array = tools["tools"].as_array().unwrap();

    let create_narrative = tools_array
        .iter()
        .find(|tool| tool["name"] == "create_narrative")
        .expect("create_narrative tool should exist");

    // Verify tool has required fields
    assert!(
        create_narrative.get("description").is_some(),
        "create_narrative should have description"
    );
    assert!(
        create_narrative.get("inputSchema").is_some(),
        "create_narrative should have inputSchema"
    );

    let input_schema = &create_narrative["inputSchema"];
    assert_eq!(input_schema["type"], "object", "inputSchema should be object");

    // Verify required parameters exist
    let properties = input_schema["properties"]
        .as_object()
        .expect("inputSchema should have properties");

    assert!(
        properties.contains_key("title"),
        "create_narrative should accept 'title' parameter"
    );
    assert!(
        properties.contains_key("description"),
        "create_narrative should accept 'description' parameter"
    );
}
