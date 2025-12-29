//! Tests for elicit_text tool.

use botticelli_mcp::{BotticelliServer, ElicitTextParams};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_elicit_text_without_dialog() {
    let server = BotticelliServer::builder().build();
    let params = ElicitTextParams {
        prompt: "Enter your name:".to_string(),
    };

    let result = server.elicit_text(Parameters(params)).await;

    // Should fail when dialog resource not configured
    assert!(
        result.is_err(),
        "Should fail when dialog resource not configured"
    );
}

#[tokio::test]
async fn test_elicit_text_params_serialization() {
    use serde_json::json;

    let json_value = json!({
        "prompt": "What is your favorite color?"
    });

    let params: ElicitTextParams =
        serde_json::from_value(json_value).expect("Should deserialize params");

    assert_eq!(params.prompt, "What is your favorite color?");
}

#[tokio::test]
async fn test_elicit_text_result_serialization() {
    use botticelli_mcp::ElicitTextResult;

    let result = ElicitTextResult::new("Blue".to_string());

    assert_eq!(result.value, "Blue");

    let json = serde_json::to_value(&result).expect("Should serialize");
    assert_eq!(json["value"], "Blue");
}
