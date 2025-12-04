use botticelli_models::{
    AnthropicClient, AnthropicContentBlock, AnthropicMessage, AnthropicRequest,
};

/// Test to discover which Anthropic models are available with current API key
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_discover_available_models() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");

    // We'll test each model individually

    // List of known Claude models to test
    let models = vec![
        "claude-3-5-sonnet-20241022",
        "claude-3-5-haiku-20241022",
        "claude-3-opus-20240229",
        "claude-3-sonnet-20240229",
        "claude-3-haiku-20240307",
        "claude-2.1",
        "claude-2.0",
        "claude-instant-1.2",
    ];

    println!("\n=== Testing Anthropic Model Availability ===\n");

    for model in models {
        let content_block = AnthropicContentBlock::Text {
            text: "Hi".to_string(),
        };

        let message = AnthropicMessage::builder()
            .role("user".to_string())
            .content(vec![content_block])
            .build()?;

        let request = AnthropicRequest::builder()
            .model(model.to_string())
            .messages(vec![message])
            .max_tokens(10u32)
            .build()?;

        print!("Testing {:<35} ... ", model);

        // Create a new client with this specific model
        let test_client = AnthropicClient::new(api_key.clone(), model);

        match test_client.generate_anthropic(&request).await {
            Ok(response) => {
                println!("✓ AVAILABLE (usage: {:?})", response.usage());
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("model_not_found") {
                    println!("✗ Not Found");
                } else if error_str.contains("permission") || error_str.contains("access") {
                    println!("✗ No Access");
                } else {
                    println!("✗ Error: {}", error_str);
                }
            }
        }
    }

    Ok(())
}
