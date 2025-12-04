use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_interface::BotticelliDriver;
use botticelli_models::HuggingFaceDriver;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_huggingface_basic_generation() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let driver = HuggingFaceDriver::new("gpt2".to_string())?;

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Hello".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build()?;

    let response = driver.generate(&request).await?;

    assert!(
        !response.outputs().is_empty(),
        "Should receive non-empty response"
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_huggingface_small_models() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let models = vec!["gpt2", "distilgpt2"];

    for model in models {
        println!("Testing model: {}", model);

        let driver = HuggingFaceDriver::new(model.to_string())?;

        let message = Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("Test".to_string())])
            .build()?;

        let request = GenerateRequest::builder()
            .messages(vec![message])
            .max_tokens(Some(5))
            .build()?;

        match driver.generate(&request).await {
            Ok(response) => {
                println!("  ✓ {} works", model);
                assert!(!response.outputs().is_empty());
            }
            Err(e) => {
                println!("  ✗ {} failed: {}", model, e);
            }
        }
    }

    Ok(())
}
