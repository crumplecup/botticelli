use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_interface::BotticelliDriver;
use botticelli_models::AnthropicClient;
use std::env;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_anthropic_simple_generation() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set for API tests");

    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text(
            "Say 'test' and nothing else.".to_string(),
        )])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .build()?;

    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response: {:?}", response.outputs());
    
    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_anthropic_with_temperature() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set for API tests");

    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Count to 3.".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .temperature(Some(0.5))
        .build()?;

    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    println!("Response with temperature: {:?}", response.outputs());
    
    Ok(())
}
