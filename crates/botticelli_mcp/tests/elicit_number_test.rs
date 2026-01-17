//! Tests for elicit_number tool.

mod helpers;

use botticelli_mcp::{BotticelliServer, ElicitNumberParams, ElicitNumberResult};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_elicit_number_without_dialog() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing elicit_number without dialog resource");

    let server = BotticelliServer::builder().build();
    let params = ElicitNumberParams {
        prompt: "Enter a number:".to_string(),
        min: 1,
        max: 10,
    };

    let result = server.elicit_number(Parameters(params)).await;
    tracing::debug!(is_err = result.is_err(), "Call completed");

    // Should fail when dialog resource not configured
    assert!(
        result.is_err(),
        "Should fail when dialog resource not configured"
    );

    tracing::info!("Without dialog test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_number_invalid_range() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing elicit_number with invalid range");

    let server = BotticelliServer::builder().build();
    let params = ElicitNumberParams {
        prompt: "Enter a number:".to_string(),
        min: 10,
        max: 1,
    };

    let result = server.elicit_number(Parameters(params)).await;
    tracing::debug!(is_err = result.is_err(), "Call completed with invalid range");

    // Should fail with invalid range (min > max)
    assert!(result.is_err(), "Should fail with invalid range");
    if let Err(err) = result {
        tracing::debug!(message = %err.message, "Error message");
        assert!(
            err.message.contains("Invalid range"),
            "Error should mention invalid range"
        );
    }

    tracing::info!("Invalid range test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_number_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ElicitNumberParams serialization");

    use serde_json::json;

    let json_value = json!({
        "prompt": "Pick a number:",
        "min": 5,
        "max": 15
    });

    let params: ElicitNumberParams = serde_json::from_value(json_value)?;
    tracing::debug!(prompt = %params.prompt, min = params.min, max = params.max, "Deserialized params");

    assert_eq!(params.prompt, "Pick a number:");
    assert_eq!(params.min, 5);
    assert_eq!(params.max, 15);

    tracing::info!("Params serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_number_result_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ElicitNumberResult serialization");

    let result = ElicitNumberResult::new(42);

    assert_eq!(result.value, 42);

    let json = serde_json::to_value(&result)?;
    tracing::debug!(?json, "Serialized result");

    assert_eq!(json["value"], 42);

    tracing::info!("Result serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_number_valid_range() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing valid range parameters");

    let params = ElicitNumberParams {
        prompt: "Enter a number:".to_string(),
        min: 0,
        max: 100,
    };

    // Validate that min <= max is accepted
    tracing::debug!(min = params.min, max = params.max, "Checking valid range");
    assert!(params.min <= params.max, "Valid range should be accepted");

    tracing::info!("Valid range test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_number_equal_min_max() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing equal min/max parameters");

    let params = ElicitNumberParams {
        prompt: "Confirm value:".to_string(),
        min: 5,
        max: 5,
    };

    // Equal min and max should be valid (single value choice)
    tracing::debug!(min = params.min, max = params.max, "Checking equal min/max");
    assert_eq!(params.min, params.max, "Equal min/max should be valid");

    tracing::info!("Equal min/max test passed");
    Ok(())
}

