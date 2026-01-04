#[cfg(feature = "anthropic")]
use botticelli_core::{GenerateRequest, Input, Message, Role};
#[cfg(feature = "anthropic")]
use botticelli_interface::BotticelliDriver;
#[cfg(feature = "anthropic")]
use botticelli_models::AnthropicClient;
#[cfg(feature = "anthropic")]
use botticelli_error::BotticelliResult;
#[cfg(feature = "anthropic")]
use std::env;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
#[cfg(feature = "anthropic")]
async fn test_anthropic_simple_generation() -> BotticelliResult<()> {
    let api_key =
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set for API tests");

    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text(
            "Say 'test' and nothing else.".to_string(),
        )])
        .build()
        .expect("Valid message");

    let request = GenerateRequest::builder().messages(vec![message]).build().expect("Valid request");

    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response: {:?}", response.outputs());

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
#[cfg(feature = "anthropic")]
async fn test_anthropic_with_temperature() -> BotticelliResult<()> {
    let api_key =
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set for API tests");

    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Count to 3.".to_string())])
        .build()
        .expect("Valid message");

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .temperature(0.5)
        .build()
        .expect("Valid request");

    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response with temperature: {:?}", response.outputs());

    Ok(())
}
