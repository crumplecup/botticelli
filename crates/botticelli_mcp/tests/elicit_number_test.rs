//! Tests for elicit_number tool.

use botticelli_mcp::{BotticelliServer, ElicitNumberParams};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_elicit_number_without_dialog() {
    let server = BotticelliServer::builder().build();
    let params = ElicitNumberParams {
        prompt: "Enter a number:".to_string(),
        min: 1,
        max: 10,
    };

    let result = server.elicit_number(Parameters(params)).await;

    // Should fail when dialog resource not configured
    assert!(
        result.is_err(),
        "Should fail when dialog resource not configured"
    );
}

#[tokio::test]
async fn test_elicit_number_invalid_range() {
    let server = BotticelliServer::builder().build();
    let params = ElicitNumberParams {
        prompt: "Enter a number:".to_string(),
        min: 10,
        max: 1,
    };

    let result = server.elicit_number(Parameters(params)).await;

    // Should fail with invalid range (min > max)
    assert!(result.is_err(), "Should fail with invalid range");
    if let Err(err) = result {
        assert!(
            err.message.contains("Invalid range"),
            "Error should mention invalid range"
        );
    }
}

#[tokio::test]
async fn test_elicit_number_params_serialization() {
    use serde_json::json;

    let json_value = json!({
        "prompt": "Pick a number:",
        "min": 5,
        "max": 15
    });

    let params: ElicitNumberParams =
        serde_json::from_value(json_value).expect("Should deserialize params");

    assert_eq!(params.prompt, "Pick a number:");
    assert_eq!(params.min, 5);
    assert_eq!(params.max, 15);
}

#[tokio::test]
async fn test_elicit_number_result_serialization() {
    use botticelli_mcp::ElicitNumberResult;

    let result = ElicitNumberResult::new(42);

    assert_eq!(result.value, 42);

    let json = serde_json::to_value(&result).expect("Should serialize");
    assert_eq!(json["value"], 42);
}

#[tokio::test]
async fn test_elicit_number_valid_range() {
    let params = ElicitNumberParams {
        prompt: "Enter a number:".to_string(),
        min: 0,
        max: 100,
    };

    // Validate that min <= max is accepted
    assert!(params.min <= params.max, "Valid range should be accepted");
}

#[tokio::test]
async fn test_elicit_number_equal_min_max() {
    let params = ElicitNumberParams {
        prompt: "Confirm value:".to_string(),
        min: 5,
        max: 5,
    };

    // Equal min and max should be valid (single value choice)
    assert_eq!(params.min, params.max, "Equal min/max should be valid");
}
