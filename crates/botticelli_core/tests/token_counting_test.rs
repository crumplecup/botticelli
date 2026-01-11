//! Token counting and cost calculation tests.

mod helpers;

use botticelli_core::{TokenUsageData, get_tokenizer};
use botticelli_error::{TokenCountingError, TokenCountingErrorKind};

#[test]
fn test_get_tokenizer() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing tokenizer retrieval for gpt-4");
    debug!(model = "gpt-4", "Getting tokenizer");

    let encoder = get_tokenizer("gpt-4")?;
    let tokens = encoder.encode_with_special_tokens("Hello, world!");

    debug!(token_count = tokens.len(), "Tokens generated");
    assert!(!tokens.is_empty());

    info!(
        model = "gpt-4",
        token_count = tokens.len(),
        "Tokenizer retrieved and tested successfully"
    );
    Ok(())
}

#[test]
fn test_get_tokenizer_invalid_model() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing tokenizer with invalid model name");
    debug!(
        model = "invalid-model-xyz-123",
        "Attempting to get tokenizer"
    );

    let result = get_tokenizer("invalid-model-xyz-123");
    assert!(result.is_err());

    if let Err(err) = result {
        let kind = err.kind();

        match kind {
            TokenCountingErrorKind::Tiktoken(boxed_err) => {
                debug!(error = %boxed_err, "Tiktoken error details");
                let error_msg = boxed_err.to_string();
                assert!(
                    error_msg.contains("invalid-model-xyz-123") || error_msg.contains("model"),
                    "Error should mention the model: {}",
                    error_msg
                );
                info!("Invalid model correctly rejected with Tiktoken error");
            }
            TokenCountingErrorKind::InvalidModel(model) => {
                debug!(model = %model, "Invalid model error");
                assert_eq!(model, "invalid-model-xyz-123");
                info!("Invalid model correctly rejected with InvalidModel error");
            }
        }
    }

    Ok(())
}

#[test]
fn test_token_usage_data_calculate_cost() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing token usage cost calculation");

    let usage = TokenUsageData::new(1_000_000, 500_000, 1_500_000);
    debug!(
        input_tokens = 1_000_000,
        output_tokens = 500_000,
        total_tokens = 1_500_000,
        "Created token usage data"
    );

    // $1 per million prompt, $2 per million completion
    let cost = usage.calculate_cost(1.0, 2.0);
    debug!(
        input_price = 1.0,
        output_price = 2.0,
        calculated_cost = cost,
        "Cost calculated"
    );

    assert!((cost - 2.0).abs() < 0.001); // 1.0 + 1.0 = 2.0
    info!(cost = cost, "Cost calculation verified");
    Ok(())
}
