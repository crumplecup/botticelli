//! Integration tests for primitive elicitation tools with InProcTransport.
//!
//! Tests Phase 3 of ELICITATION_MCP_INTEGRATION_PLAN.md:
//! - Primitive tools work with mock dialog
//! - InProcTransport enables in-process communication
//! - Full MCP protocol flow from client to dialog

use async_trait::async_trait;
use botticelli_error::BotticelliResult;
use botticelli_mcp::{DialogResource, ElicitationDialog, InProcTransport};
use pmcp::{Client, ClientCapabilities, Server};
use serde_json::json;
use std::sync::Arc;

/// Mock dialog for testing with preset responses.
#[derive(Debug)]
struct MockDialog {
    text_response: String,
    choice_response: usize,
    number_response: i64,
    bool_response: bool,
}

impl MockDialog {
    fn new() -> Self {
        Self {
            text_response: "Mock text response".to_string(),
            choice_response: 1,
            number_response: 42,
            bool_response: true,
        }
    }

    fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text_response = text.into();
        self
    }

    fn with_choice(mut self, index: usize) -> Self {
        self.choice_response = index;
        self
    }

    fn with_number(mut self, num: i64) -> Self {
        self.number_response = num;
        self
    }

    fn with_bool(mut self, val: bool) -> Self {
        self.bool_response = val;
        self
    }
}

#[async_trait]
impl ElicitationDialog for MockDialog {
    async fn ask_text(&mut self, _prompt: &str) -> BotticelliResult<String> {
        Ok(self.text_response.clone())
    }

    async fn ask_confirmation(&mut self, _prompt: &str, _default: bool) -> BotticelliResult<bool> {
        Ok(self.bool_response)
    }

    async fn ask_choice(&mut self, _prompt: &str, _options: &[&str]) -> BotticelliResult<usize> {
        Ok(self.choice_response)
    }

    async fn ask_number(&mut self, _prompt: &str, _min: i64, _max: i64) -> BotticelliResult<i64> {
        Ok(self.number_response)
    }

    async fn ask_file_path(&mut self, _prompt: &str) -> BotticelliResult<String> {
        Ok("/mock/path.txt".to_string())
    }

    async fn show_info(&mut self, _message: &str) -> BotticelliResult<()> {
        Ok(())
    }

    async fn show_warning(&mut self, _message: &str) -> BotticelliResult<()> {
        Ok(())
    }

    async fn show_error(&mut self, _message: &str) -> BotticelliResult<()> {
        Ok(())
    }

    async fn show_validation(&mut self, _validation_text: &str) -> BotticelliResult<()> {
        Ok(())
    }

    async fn show_progress(
        &mut self,
        _current: usize,
        _total: usize,
        _description: &str,
    ) -> BotticelliResult<()> {
        Ok(())
    }

    async fn show_preview(&mut self, _toml: &str) -> BotticelliResult<()> {
        Ok(())
    }
}

/// Helper to extract tool result from pmcp CallToolResult.
fn extract_tool_result(result: &pmcp::types::CallToolResult) -> serde_json::Value {
    assert!(!result.content.is_empty(), "Expected content in response");

    // The response content is a JSON string containing our tool result
    let content_json = &result.content[0];
    // Convert Content to JSON to extract the text field
    let content_str = serde_json::to_string(&content_json)
        .expect("Failed to serialize content");
    let content_val: serde_json::Value = serde_json::from_str(&content_str)
        .expect("Failed to parse content");

    // Extract text field and parse our tool result
    // Note: After fixing elicitation primitive tools, the result is now the raw value
    // directly (e.g., "hello" or 42 or true) instead of {"value": ...}
    let result_text = content_val["text"].as_str()
        .expect("Expected text field in content");
    serde_json::from_str(result_text)
        .expect("Failed to parse tool result")
}

/// Helper to create a server with primitive elicitation tools.
fn build_server_with_dialog(dialog: Arc<DialogResource>) -> Server {
    use botticelli_mcp::register_all_tools;

    let builder = Server::builder()
        .name("test-elicitation-server")
        .version("0.1.0")
        .capabilities(pmcp::types::capabilities::ServerCapabilities::tools_only());

    let builder = register_all_tools(
        builder,
        Some(dialog),
        #[cfg(feature = "database")]
        None,
    );

    builder.build().expect("Failed to build server")
}

#[tokio::test]
async fn test_elicit_text_integration() {
    let _ = tracing_subscriber::fmt::try_init();

    // Create mock dialog with custom response
    let mock_dialog = MockDialog::new().with_text("Hello from test");
    let dialog_resource = Arc::new(DialogResource::new(Box::new(mock_dialog)));

    // Build server with elicitation tools
    let server = build_server_with_dialog(dialog_resource);

    // Create in-process transport
    let (client_transport, server_transport) = InProcTransport::pair();
    let _server_handle = InProcTransport::spawn_server(server, server_transport);

    // Create and initialize client
    let mut client = Client::new(client_transport);
    client
        .initialize(ClientCapabilities::minimal())
        .await
        .expect("Failed to initialize client");

    // Call elicit_text tool
    let result = client
        .call_tool(
            "elicit_text".to_string(),
            json!({ "prompt": "Enter some text:" }),
        )
        .await
        .expect("Failed to call elicit_text");

    // Verify response
    let tool_result = extract_tool_result(&result);
    assert_eq!(tool_result, "Hello from test");
}

#[tokio::test]
async fn test_elicit_select_integration() {
    let _ = tracing_subscriber::fmt::try_init();

    // Create mock dialog that selects second option (index 1)
    let mock_dialog = MockDialog::new().with_choice(1);
    let dialog_resource = Arc::new(DialogResource::new(Box::new(mock_dialog)));

    // Build server
    let server = build_server_with_dialog(dialog_resource);

    // Create transport and client
    let (client_transport, server_transport) = InProcTransport::pair();
    let _server_handle = InProcTransport::spawn_server(server, server_transport);

    let mut client = Client::new(client_transport);
    client
        .initialize(ClientCapabilities::minimal())
        .await
        .expect("Failed to initialize");

    // Call elicit_select tool
    let result = client
        .call_tool(
            "elicit_select".to_string(),
            json!({
                "prompt": "Choose one:",
                "options": ["Option A", "Option B", "Option C"]
            }),
        )
        .await
        .expect("Failed to call elicit_select");

    // Verify selected option
    let tool_result = extract_tool_result(&result);
    assert_eq!(tool_result, "Option B");
}

#[tokio::test]
async fn test_elicit_number_integration() {
    let _ = tracing_subscriber::fmt::try_init();

    // Create mock dialog that returns 75
    let mock_dialog = MockDialog::new().with_number(75);
    let dialog_resource = Arc::new(DialogResource::new(Box::new(mock_dialog)));

    // Build server
    let server = build_server_with_dialog(dialog_resource);

    // Create transport and client
    let (client_transport, server_transport) = InProcTransport::pair();
    let _server_handle = InProcTransport::spawn_server(server, server_transport);

    let mut client = Client::new(client_transport);
    client
        .initialize(ClientCapabilities::minimal())
        .await
        .expect("Failed to initialize");

    // Call elicit_number tool
    let result = client
        .call_tool(
            "elicit_number".to_string(),
            json!({
                "prompt": "Enter a number:",
                "min": 0,
                "max": 100
            }),
        )
        .await
        .expect("Failed to call elicit_number");

    // Verify number
    let tool_result = extract_tool_result(&result);
    assert_eq!(tool_result, 75);
}

#[tokio::test]
async fn test_elicit_bool_integration() {
    let _ = tracing_subscriber::fmt::try_init();

    // Create mock dialog that returns false
    let mock_dialog = MockDialog::new().with_bool(false);
    let dialog_resource = Arc::new(DialogResource::new(Box::new(mock_dialog)));

    // Build server
    let server = build_server_with_dialog(dialog_resource);

    // Create transport and client
    let (client_transport, server_transport) = InProcTransport::pair();
    let _server_handle = InProcTransport::spawn_server(server, server_transport);

    let mut client = Client::new(client_transport);
    client
        .initialize(ClientCapabilities::minimal())
        .await
        .expect("Failed to initialize");

    // Call elicit_bool tool
    let result = client
        .call_tool(
            "elicit_bool".to_string(),
            json!({
                "prompt": "Confirm action?",
                "default": true
            }),
        )
        .await
        .expect("Failed to call elicit_bool");

    // Verify boolean
    let tool_result = extract_tool_result(&result);
    assert_eq!(tool_result, false);
}

#[tokio::test]
async fn test_all_primitive_tools_available() {
    let _ = tracing_subscriber::fmt::try_init();

    // Create server with dialog
    let mock_dialog = MockDialog::new();
    let dialog_resource = Arc::new(DialogResource::new(Box::new(mock_dialog)));
    let server = build_server_with_dialog(dialog_resource);

    // Create transport and client
    let (client_transport, server_transport) = InProcTransport::pair();
    let _server_handle = InProcTransport::spawn_server(server, server_transport);

    let mut client = Client::new(client_transport);
    client
        .initialize(ClientCapabilities::minimal())
        .await
        .expect("Failed to initialize");

    // List tools
    let tools_result = client
        .list_tools(None)
        .await
        .expect("Failed to list tools");

    let tool_names: Vec<&str> = tools_result
        .tools
        .iter()
        .map(|t| t.name.as_str())
        .collect();

    // Verify all primitive elicitation tools are present
    assert!(
        tool_names.contains(&"elicit_text"),
        "elicit_text tool not found"
    );
    assert!(
        tool_names.contains(&"elicit_select"),
        "elicit_select tool not found"
    );
    assert!(
        tool_names.contains(&"elicit_number"),
        "elicit_number tool not found"
    );
    assert!(
        tool_names.contains(&"elicit_bool"),
        "elicit_bool tool not found"
    );
}

#[tokio::test]
async fn test_sequential_tool_calls() {
    let _ = tracing_subscriber::fmt::try_init();

    // Create server with dialog
    let mock_dialog = MockDialog::new()
        .with_text("First response")
        .with_number(99)
        .with_bool(true);
    let dialog_resource = Arc::new(DialogResource::new(Box::new(mock_dialog)));
    let server = build_server_with_dialog(dialog_resource);

    // Create transport and client
    let (client_transport, server_transport) = InProcTransport::pair();
    let _server_handle = InProcTransport::spawn_server(server, server_transport);

    let mut client = Client::new(client_transport);
    client
        .initialize(ClientCapabilities::minimal())
        .await
        .expect("Failed to initialize");

    // Make multiple sequential calls
    let text_result = client
        .call_tool(
            "elicit_text".to_string(),
            json!({ "prompt": "Enter text:" }),
        )
        .await
        .expect("Failed to call elicit_text");
    assert!(text_result.content.len() > 0);

    let number_result = client
        .call_tool(
            "elicit_number".to_string(),
            json!({ "prompt": "Enter number:", "min": 0, "max": 100 }),
        )
        .await
        .expect("Failed to call elicit_number");
    assert!(number_result.content.len() > 0);

    let bool_result = client
        .call_tool(
            "elicit_bool".to_string(),
            json!({ "prompt": "Confirm?", "default": false }),
        )
        .await
        .expect("Failed to call elicit_bool");
    assert!(bool_result.content.len() > 0);
}
