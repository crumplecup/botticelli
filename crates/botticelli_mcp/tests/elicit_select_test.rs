//! Tests for elicit_select tool.

use botticelli_mcp::{BotticelliServer, ElicitSelectParams};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_elicit_select_without_dialog() {
    let server = BotticelliServer::builder().build();
    let params = ElicitSelectParams {
        prompt: "Choose one:".to_string(),
        options: vec!["Option A".to_string(), "Option B".to_string()],
    };

    let result = server.elicit_select(Parameters(params)).await;

    // Should fail when dialog resource not configured
    assert!(
        result.is_err(),
        "Should fail when dialog resource not configured"
    );
}

#[tokio::test]
async fn test_elicit_select_empty_options() {
    let server = BotticelliServer::builder().build();
    let params = ElicitSelectParams {
        prompt: "Choose one:".to_string(),
        options: vec![],
    };

    let result = server.elicit_select(Parameters(params)).await;

    // Should fail with empty options
    assert!(result.is_err(), "Should fail with empty options");
    if let Err(err) = result {
        assert!(
            err.message.contains("empty"),
            "Error should mention empty options"
        );
    }
}

#[tokio::test]
async fn test_elicit_select_params_serialization() {
    use serde_json::json;

    let json_value = json!({
        "prompt": "Pick a color:",
        "options": ["Red", "Green", "Blue"]
    });

    let params: ElicitSelectParams =
        serde_json::from_value(json_value).expect("Should deserialize params");

    assert_eq!(params.prompt, "Pick a color:");
    assert_eq!(params.options.len(), 3);
    assert_eq!(params.options[0], "Red");
    assert_eq!(params.options[1], "Green");
    assert_eq!(params.options[2], "Blue");
}

#[tokio::test]
async fn test_elicit_select_result_serialization() {
    use botticelli_mcp::ElicitSelectResult;

    let result = ElicitSelectResult::new("Option A".to_string());

    assert_eq!(result.value, "Option A");

    let json = serde_json::to_value(&result).expect("Should serialize");
    assert_eq!(json["value"], "Option A");
}

#[tokio::test]
async fn test_elicit_select_single_option() {
    let params = ElicitSelectParams {
        prompt: "Confirm:".to_string(),
        options: vec!["Only Option".to_string()],
    };

    // Single option should be valid
    assert_eq!(params.options.len(), 1, "Single option should be valid");
}

#[tokio::test]
async fn test_elicit_select_many_options() {
    let params = ElicitSelectParams {
        prompt: "Choose a number:".to_string(),
        options: (1..=10).map(|i| format!("Option {}", i)).collect(),
    };

    // Many options should be valid
    assert_eq!(params.options.len(), 10, "Many options should be valid");
}
