use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_interface::BotticelliDriver;
use botticelli_models::HuggingFaceClient;
use std::env;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_huggingface_basic_inference() -> Result<(), Box<dyn std::error::Error>> {
    // Load API key from environment
    dotenvy::dotenv().ok();
    let api_key = env::var("HUGGINGFACE_API_KEY")
        .expect("HUGGINGFACE_API_KEY must be set in .env");

    // Create client
    let client = HuggingFaceClient::new(api_key)?;

    // Use a small, free model for testing
    let model = "gpt2"; // Small, widely available model
    
    // Build request using our standard interface
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Hello, world!".to_string())])
        .build()?;
    
    let request = GenerateRequest::builder()
        .model(Some(model.to_string()))
        .messages(vec![message])
        .max_tokens(Some(10))
        .build()?;

    // Make request
    let response = client.generate(&request).await?;

    println!("Response: {:?}", response.outputs());
    assert!(!response.outputs().is_empty(), "Should receive non-empty response");

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_huggingface_model_availability() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let api_key = env::var("HUGGINGFACE_API_KEY")
        .expect("HUGGINGFACE_API_KEY must be set in .env");

    let client = HuggingFaceClient::new(api_key)?;

    // Test various free/small models
    let models = vec![
        "gpt2",
        "distilgpt2",
        "facebook/opt-125m",
    ];

    for model in models {
        println!("Testing model: {}", model);
        
        let message = Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("Test".to_string())])
            .build()?;
        
        let request = GenerateRequest::builder()
            .model(Some(model.to_string()))
            .messages(vec![message])
            .max_tokens(Some(5))
            .build()?;
        
        match client.generate(&request).await {
            Ok(response) => {
                println!("  ✓ {} works: {:?}", model, response.outputs());
            }
            Err(e) => {
                println!("  ✗ {} failed: {}", model, e);
            }
        }
    }

    Ok(())
}
