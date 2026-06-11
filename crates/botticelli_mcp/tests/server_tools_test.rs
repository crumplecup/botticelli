//! Integration tests for BotticelliServer via in-process transport.
//!
//! Uses tokio::io::duplex to run client and server in the same process
//! without any network or subprocess overhead.

use botticelli_mcp::BotticelliServer;
use rmcp::handler::client::ClientHandler;
use rmcp::model::{
    CallToolRequestParams, ClientCapabilities, Implementation, InitializeRequestParams,
    ProtocolVersion,
};
use tokio::io::split;

/// Minimal client handler for in-process tests — no elicitation.
#[derive(Clone, Default)]
struct TestHandler;

impl ClientHandler for TestHandler {
    #[tracing::instrument(skip(self))]
    fn get_info(&self) -> InitializeRequestParams {
        InitializeRequestParams::new(
            ClientCapabilities::default(),
            Implementation::new("test-client", "0.0.0"),
        )
        .with_protocol_version(ProtocolVersion::V_2025_06_18)
    }
}

/// Start an in-process MCP server. The returned service must stay alive for the
/// connection to persist (dropping it closes the channel).
#[tracing::instrument]
async fn in_proc_server() -> rmcp::service::RunningService<rmcp::RoleClient, TestHandler> {
    let (client_stream, server_stream) = tokio::io::duplex(65536);
    let (sr, sw) = split(server_stream);
    let (cr, cw) = split(client_stream);
    tokio::spawn(async move {
        if let Ok(svc) = rmcp::service::serve_server(BotticelliServer::new(), (sr, sw)).await {
            svc.waiting().await.ok();
        }
    });
    rmcp::serve_client(TestHandler, (cr, cw))
        .await
        .expect("in-proc MCP handshake failed")
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

#[tokio::test]
async fn test_list_tools_includes_core_tools() {
    let svc = in_proc_server().await;
    let tools = svc
        .peer()
        .list_tools(Default::default())
        .await
        .expect("list_tools failed");
    let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        names.contains(&"echo"),
        "echo should be listed; got: {names:?}"
    );
    assert!(
        names.contains(&"get_server_info"),
        "get_server_info should be listed; got: {names:?}"
    );
    assert!(
        names.contains(&"validate_narrative"),
        "validate_narrative should be listed; got: {names:?}"
    );
    assert!(
        names.contains(&"generate_narrative_toml"),
        "generate_narrative_toml should be listed; got: {names:?}"
    );
}

#[tokio::test]
async fn test_echo_returns_input() {
    let svc = in_proc_server().await;
    let args: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(serde_json::json!({ "message": "ping from test" })).unwrap();
    let result = svc
        .peer()
        .call_tool(CallToolRequestParams::new("echo").with_arguments(args))
        .await
        .expect("echo call failed");
    let text = content_text(&result);
    assert!(
        text.contains("ping from test"),
        "echo should return input message; got: {text}"
    );
}

#[tokio::test]
async fn test_get_server_info_returns_json() {
    let svc = in_proc_server().await;
    let result = svc
        .peer()
        .call_tool(CallToolRequestParams::new("get_server_info"))
        .await
        .expect("get_server_info call failed");
    let text = content_text(&result);
    let json: serde_json::Value =
        serde_json::from_str(&text).expect("get_server_info should return JSON");
    assert!(
        json["name"].is_string(),
        "response should have name field; got: {json}"
    );
    assert!(
        json["version"].is_string(),
        "response should have version field; got: {json}"
    );
}

#[tokio::test]
async fn test_validate_narrative_valid_toml() {
    let svc = in_proc_server().await;
    let toml = r#"
[narrative]
name = "test_workflow"
description = "A minimal test narrative"

[toc]
order = ["step_one"]

[acts]
step_one = "Perform the first step"
"#;
    let args: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(serde_json::json!({ "content": toml })).unwrap();
    let result = svc
        .peer()
        .call_tool(CallToolRequestParams::new("validate_narrative").with_arguments(args))
        .await
        .expect("validate_narrative call failed");
    let text = content_text(&result);
    let json: serde_json::Value =
        serde_json::from_str(&text).expect("validate_narrative should return JSON");
    assert_eq!(
        json["valid"],
        serde_json::Value::Bool(true),
        "valid TOML should pass validation; got: {json}"
    );
    assert_eq!(
        json["errors"].as_array().map(|a| a.len()),
        Some(0),
        "valid TOML should have no errors; got: {json}"
    );
}

#[tokio::test]
async fn test_validate_narrative_invalid_toml_returns_errors() {
    let svc = in_proc_server().await;
    let invalid_toml = "not = valid {{ toml }}";
    let args: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(serde_json::json!({ "content": invalid_toml })).unwrap();
    let result = svc
        .peer()
        .call_tool(CallToolRequestParams::new("validate_narrative").with_arguments(args))
        .await
        .expect("validate_narrative call failed");
    let text = content_text(&result);
    let json: serde_json::Value =
        serde_json::from_str(&text).expect("validate_narrative should return JSON");
    assert_eq!(
        json["valid"],
        serde_json::Value::Bool(false),
        "invalid TOML should fail validation; got: {json}"
    );
}

#[tokio::test]
async fn test_generate_narrative_toml_produces_acts() {
    let svc = in_proc_server().await;
    let args: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(serde_json::json!({
            "name": "my_workflow",
            "description": "First gather requirements, then write code, then review it",
        }))
        .unwrap();
    let result = svc
        .peer()
        .call_tool(CallToolRequestParams::new("generate_narrative_toml").with_arguments(args))
        .await
        .expect("generate_narrative_toml call failed");
    let text = content_text(&result);
    let json: serde_json::Value =
        serde_json::from_str(&text).expect("generate_narrative_toml should return JSON");
    assert!(
        json["toml"].is_string(),
        "response should have toml field; got: {json}"
    );
    assert!(
        json["act_count"].as_i64().unwrap_or(0) > 0,
        "should have at least one act; got: {json}"
    );
}
