//! Tests for elicit_text tool.

mod helpers;

use botticelli_mcp::{BotticelliServer, ElicitTextParams, ElicitTextResult};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_elicit_text_without_dialog() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing elicit_text without dialog resource");

    let server = BotticelliServer::builder().build();
    tracing::debug!("Created server without dialog");

    let params = ElicitTextParams::new("Enter your name:".to_string());

    let result = server.elicit_text(Parameters(params)).await;
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
async fn test_elicit_text_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ElicitTextParams serialization");

    use serde_json::json;

    let json_value = json!({
        "prompt": "What is your favorite color?"
    });

    let params: ElicitTextParams = serde_json::from_value(json_value)?;
    tracing::debug!(prompt = %params.prompt(), "Deserialized params");

    assert_eq!(params.prompt(), "What is your favorite color?");

    tracing::info!("Params serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_text_result_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ElicitTextResult serialization");

    let result = ElicitTextResult::new("Blue".to_string());

    assert_eq!(result.value(), "Blue");

    let json = serde_json::to_value(&result)?;
    tracing::debug!(?json, "Serialized result");

    assert_eq!(json["value"], "Blue");

    tracing::info!("Result serialization test passed");
    Ok(())
}

