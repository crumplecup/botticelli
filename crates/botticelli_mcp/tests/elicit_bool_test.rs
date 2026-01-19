//! Tests for elicit_bool tool.

mod helpers;

use botticelli_mcp::{BotticelliServer, ElicitBoolParams, ElicitBoolResult};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_elicit_bool_without_dialog() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing elicit_bool without dialog resource");

    let server = BotticelliServer::builder().build()?;
    let params = ElicitBoolParams::new("Do you agree?".to_string(), false);

    let result = server.elicit_bool(Parameters(params)).await;
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
async fn test_elicit_bool_params_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ElicitBoolParams serialization");

    use serde_json::json;

    let json_value = json!({
        "prompt": "Continue?",
        "default": true
    });

    let params: ElicitBoolParams = serde_json::from_value(json_value)?;
    tracing::debug!(prompt = %params.prompt(), default = params.default(), "Deserialized params");

    assert_eq!(params.prompt(), "Continue?");
    assert_eq!(*params.default(), true);

    tracing::info!("Params serialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_bool_params_default_value() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ElicitBoolParams default value");

    use serde_json::json;

    let json_value = json!({
        "prompt": "Continue?"
    });

    let params: ElicitBoolParams = serde_json::from_value(json_value)?;
    tracing::debug!(default = params.default(), "Deserialized with default");

    assert_eq!(params.prompt(), "Continue?");
    assert_eq!(*params.default(), false, "Should default to false");

    tracing::info!("Default value test passed");
    Ok(())
}

#[tokio::test]
async fn test_elicit_bool_result_serialization() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing ElicitBoolResult serialization");

    let result = ElicitBoolResult::new(true);

    assert_eq!(*result.value(), true);

    let json = serde_json::to_value(&result)?;
    tracing::debug!(?json, "Serialized result");

    assert_eq!(json["value"], true);

    tracing::info!("Result serialization test passed");
    Ok(())
}
