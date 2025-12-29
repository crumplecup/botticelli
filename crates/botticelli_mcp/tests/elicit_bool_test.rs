//! Tests for elicit_bool tool.

use botticelli_mcp::{BotticelliServer, ElicitBoolParams};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_elicit_bool_without_dialog() {
    let server = BotticelliServer::builder().build();
    let params = ElicitBoolParams {
        prompt: "Do you agree?".to_string(),
        default: false,
    };

    let result = server.elicit_bool(Parameters(params)).await;

    // Should fail when dialog resource not configured
    assert!(
        result.is_err(),
        "Should fail when dialog resource not configured"
    );
}

#[tokio::test]
async fn test_elicit_bool_params_serialization() {
    use serde_json::json;

    let json_value = json!({
        "prompt": "Continue?",
        "default": true
    });

    let params: ElicitBoolParams =
        serde_json::from_value(json_value).expect("Should deserialize params");

    assert_eq!(params.prompt, "Continue?");
    assert_eq!(params.default, true);
}

#[tokio::test]
async fn test_elicit_bool_params_default_value() {
    use serde_json::json;

    let json_value = json!({
        "prompt": "Continue?"
    });

    let params: ElicitBoolParams =
        serde_json::from_value(json_value).expect("Should deserialize with default");

    assert_eq!(params.prompt, "Continue?");
    assert_eq!(params.default, false, "Should default to false");
}

#[tokio::test]
async fn test_elicit_bool_result_serialization() {
    use botticelli_mcp::ElicitBoolResult;

    let result = ElicitBoolResult::new(true);

    assert_eq!(result.value, true);

    let json = serde_json::to_value(&result).expect("Should serialize");
    assert_eq!(json["value"], true);
}
