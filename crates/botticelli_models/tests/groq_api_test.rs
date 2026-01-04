#[cfg(feature = "groq")]
use botticelli_core::{GenerateRequest, Input, Message, Role};
#[cfg(feature = "groq")]
use botticelli_interface::BotticelliDriver;
#[cfg(feature = "groq")]
use botticelli_models::GroqDriver;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
#[cfg(feature = "groq")]
async fn test_groq_basic_generation() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let driver = GroqDriver::new("llama-3.1-8b-instant".to_string())?;

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Hello".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(10u32)
        .build()?;

    let response = driver.generate(&request).await?;

    assert!(
        !response.outputs().is_empty(),
        "Should receive non-empty response"
    );
    println!("Response: {:?}", response.outputs());

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
#[cfg(feature = "groq")]
async fn test_groq_small_models() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let models = vec!["llama-3.1-8b-instant", "llama-3.3-70b-versatile"];

    for model in models {
        println!("Testing model: {}", model);

        let driver = GroqDriver::new(model.to_string())?;

        let message = Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("Hi".to_string())])
            .build()?;

        let request = GenerateRequest::builder()
            .messages(vec![message])
            .max_tokens(5u32)
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
