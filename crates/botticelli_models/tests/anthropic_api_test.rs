#[cfg(feature = "anthropic")]
use botticelli_core::{GenerateRequest, Input, Message, Role};
#[cfg(feature = "anthropic")]
use botticelli_interface::BotticelliDriver;
#[cfg(feature = "anthropic")]
use botticelli_models::AnthropicClient;
#[cfg(feature = "anthropic")]
use botticelli_error::{AnthropicErrorKind, BotticelliResult, ModelsErrorKind};
#[cfg(feature = "anthropic")]
use std::env;
#[cfg(feature = "anthropic")]
use std::sync::Arc;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug")),
        )
        .try_init();
}

#[tokio::test]
#[cfg(feature = "api")]
#[cfg(feature = "anthropic")]
async fn test_anthropic_simple_generation() -> BotticelliResult<()> {
    init_tracing();

    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|e| botticelli_error::AnthropicErrorKind::EnvVar(Arc::new(e)))?;

    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text(
            "Say 'test' and nothing else".to_string(),
        )])
        .build()
        .map_err(|e| ModelsErrorKind::Builder(e.to_string()))?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .build()
        .map_err(|e| ModelsErrorKind::Builder(e.to_string()))?;

    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response: {:?}", response.outputs());

    Ok(())
}

#[tokio::test]
#[cfg(feature = "api")]
#[cfg(feature = "anthropic")]
async fn test_anthropic_with_temperature() -> BotticelliResult<()> {
    init_tracing();

    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|e| botticelli_error::AnthropicErrorKind::EnvVar(Arc::new(e)))?;

    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Count to 3.".to_string())])
        .build()
        .map_err(|e| AnthropicErrorKind::Builder(e.to_string()))?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .temperature(0.5)
        .build()
        .map_err(|e| AnthropicErrorKind::Builder(e.to_string()))?;

    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response with temperature: {:?}", response.outputs());

    Ok(())
}
