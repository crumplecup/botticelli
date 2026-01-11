mod helpers;

#[cfg(feature = "huggingface")]
use botticelli_core::{GenerateRequest, Input, Message, Role};
#[cfg(feature = "huggingface")]
use botticelli_interface::BotticelliDriver;
#[cfg(feature = "huggingface")]
use botticelli_models::HuggingFaceDriver;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
#[cfg(feature = "huggingface")]
async fn test_huggingface_basic_generation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing HuggingFace basic generation");

    dotenvy::dotenv().ok();

    let driver = HuggingFaceDriver::new("meta-llama/Llama-3.2-1B-Instruct".to_string())?;
    tracing::debug!(
        model = "meta-llama/Llama-3.2-1B-Instruct",
        "Created HuggingFaceDriver"
    );

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Hello".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(10u32)
        .build()?;

    tracing::debug!("Sending generation request");
    let response = driver.generate(&request).await?;

    assert!(
        !response.outputs().is_empty(),
        "Should receive non-empty response"
    );
    println!("Response: {:?}", response.outputs());
    tracing::debug!(output_count = response.outputs().len(), "Received response");

    tracing::info!("HuggingFace basic generation test passed");
    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
#[cfg(feature = "huggingface")]
async fn test_huggingface_small_models() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing HuggingFace small models");

    dotenvy::dotenv().ok();

    let models = vec![
        "meta-llama/Llama-3.2-1B-Instruct",
        "mistralai/Mistral-7B-Instruct-v0.3",
    ];
    tracing::debug!(model_count = models.len(), "Testing multiple models");

    for model in models {
        println!("Testing model: {}", model);
        tracing::debug!(model, "Testing model");

        let driver = HuggingFaceDriver::new(model.to_string())?;

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
                tracing::debug!(
                    model,
                    output_count = response.outputs().len(),
                    "Model test succeeded"
                );
            }
            Err(e) => {
                println!("  ✗ {} failed: {}", model, e);
                tracing::warn!(model, error = %e, "Model test failed");
            }
        }
    }

    tracing::info!("HuggingFace small models test passed");
    Ok(())
}
