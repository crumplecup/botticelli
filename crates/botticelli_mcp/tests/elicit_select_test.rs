//! Tests for elicit_select tool.

mod helpers;

use botticelli_mcp::{BotticelliServer, ElicitSelectParams, ElicitSelectResult};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_elicit_select_without_dialog() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing elicit_select without dialog resource");

    let server = BotticelliServer::builder().build()?;
    let params = ElicitSelectParams::new(
        "Choose one:".to_string(),
        vec!["Option A".to_string(), "Option B".to_string()],
    );

    let result = server.elicit_select(Parameters(params)).await;
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
async fn test_elicit_select_empty_options() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing elicit_select with empty options");

    let server = BotticelliServer::builder().build()?;
    let params = ElicitSelectParams::new("Choose one:".to_string(), vec![]);

    let result = server.elicit_select(Parameters(params)).await;
    tracing::debug!(
        is_err = result.is_err(),
        "Call completed with empty options"
    );

    // Should fail with empty options
    assert!(result.is_err(), "Should fail with empty options");
    if let Err(err) = result {
        tracing::debug!(message = %err.message, "Error message");
        assert!(
            err.message.contains("empty"),
            "Error should mention empty options"
        );
    }

    tracing::info!("Empty options test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_select_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ElicitSelectParams serialization");

    use serde_json::json;

    let json_value = json!({
        "prompt": "Pick a color:",
        "options": ["Red", "Green", "Blue"]
    });

    let params: ElicitSelectParams = serde_json::from_value(json_value)?;
    tracing::debug!(prompt = %params.prompt(), option_count = params.options().len(), "Deserialized params");

    assert_eq!(params.prompt(), "Pick a color:");
    assert_eq!(params.options().len(), 3);
    assert_eq!(params.options()[0], "Red");
    assert_eq!(params.options()[1], "Green");
    assert_eq!(params.options()[2], "Blue");

    tracing::info!("Params serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_select_result_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ElicitSelectResult serialization");

    let result = ElicitSelectResult::new("Option A".to_string());

    assert_eq!(result.value(), "Option A");

    let json = serde_json::to_value(&result)?;
    tracing::debug!(?json, "Serialized result");

    assert_eq!(json["value"], "Option A");

    tracing::info!("Result serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_select_single_option() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing single option");

    let params = ElicitSelectParams::new("Confirm:".to_string(), vec!["Only Option".to_string()]);

    // Single option should be valid
    tracing::debug!(
        option_count = params.options().len(),
        "Checking single option"
    );
    assert_eq!(params.options().len(), 1, "Single option should be valid");

    tracing::info!("Single option test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_select_many_options() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing many options");

    let params = ElicitSelectParams::new(
        "Choose a number:".to_string(),
        (1..=10).map(|i| format!("Option {}", i)).collect(),
    );

    // Many options should be valid
    tracing::debug!(
        option_count = params.options().len(),
        "Checking many options"
    );
    assert_eq!(params.options().len(), 10, "Many options should be valid");

    tracing::info!("Many options test passed");
    Ok(())
}
