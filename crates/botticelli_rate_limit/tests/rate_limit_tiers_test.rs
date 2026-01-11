//! Tests for rate limit tiers.

mod helpers;

use botticelli_interface::Tier;

#[cfg(feature = "gemini")]
use botticelli_rate_limit::GeminiTier;

#[cfg(feature = "anthropic")]
use botticelli_rate_limit::AnthropicTier;

use botticelli_rate_limit::OpenAITier;

#[cfg(feature = "gemini")]
#[test]
fn test_gemini_free_tier() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Gemini Free tier");

    let tier = GeminiTier::Free;

    tracing::debug!(rpm = ?tier.rpm(), "Checking RPM");
    assert_eq!(tier.rpm(), Some(10));

    tracing::debug!(tpm = ?tier.tpm(), "Checking TPM");
    assert_eq!(tier.tpm(), Some(250_000));

    tracing::debug!(rpd = ?tier.rpd(), "Checking RPD");
    assert_eq!(tier.rpd(), Some(250));

    assert_eq!(tier.max_concurrent(), Some(1));
    assert_eq!(tier.cost_per_million_input_tokens(), Some(0.0));
    assert_eq!(tier.cost_per_million_output_tokens(), Some(0.0));
    assert_eq!(tier.daily_quota_usd(), None);
    assert_eq!(tier.name(), "Free");

    tracing::info!("Gemini Free tier test passed");
    Ok(())
}

#[cfg(feature = "gemini")]
#[test]
fn test_gemini_payasyougo_tier() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Gemini PayAsYouGo tier");

    let tier = GeminiTier::PayAsYouGo;

    tracing::debug!(rpm = ?tier.rpm(), tpm = ?tier.tpm(), "Checking rate limits");
    assert_eq!(tier.rpm(), Some(360));
    assert_eq!(tier.tpm(), Some(4_000_000));
    assert_eq!(tier.rpd(), None);
    assert_eq!(tier.max_concurrent(), Some(1));
    assert_eq!(tier.cost_per_million_input_tokens(), Some(0.075));
    assert_eq!(tier.cost_per_million_output_tokens(), Some(0.30));
    assert_eq!(tier.name(), "Pay-as-you-go");

    tracing::info!("Gemini PayAsYouGo tier test passed");
    Ok(())
}

#[cfg(feature = "anthropic")]
#[test]
fn test_anthropic_tier1() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Anthropic Tier 1");

    let tier = AnthropicTier::Tier1;

    tracing::debug!(rpm = ?tier.rpm(), tpm = ?tier.tpm(), "Checking rate limits");
    assert_eq!(tier.rpm(), Some(5));
    assert_eq!(tier.tpm(), Some(20_000));
    assert_eq!(tier.rpd(), None);
    assert_eq!(tier.max_concurrent(), Some(5));
    assert_eq!(tier.cost_per_million_input_tokens(), Some(3.0));
    assert_eq!(tier.cost_per_million_output_tokens(), Some(15.0));
    assert_eq!(tier.name(), "Tier 1");

    tracing::info!("Anthropic Tier 1 test passed");
    Ok(())
}

#[cfg(feature = "anthropic")]
#[test]
fn test_anthropic_tier4() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Anthropic Tier 4");

    let tier = AnthropicTier::Tier4;

    tracing::debug!(rpm = ?tier.rpm(), tpm = ?tier.tpm(), "Checking rate limits");
    assert_eq!(tier.rpm(), Some(2000));
    assert_eq!(tier.tpm(), Some(160_000));
    assert_eq!(tier.max_concurrent(), Some(5));
    assert_eq!(tier.name(), "Tier 4");

    tracing::info!("Anthropic Tier 4 test passed");
    Ok(())
}

#[test]
fn test_openai_free_tier() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing OpenAI Free tier");

    let tier = OpenAITier::Free;

    tracing::debug!(rpm = ?tier.rpm(), tpm = ?tier.tpm(), rpd = ?tier.rpd(), "Checking rate limits");
    assert_eq!(tier.rpm(), Some(3));
    assert_eq!(tier.tpm(), Some(40_000));
    assert_eq!(tier.rpd(), Some(200));
    assert_eq!(tier.max_concurrent(), Some(50));
    assert_eq!(tier.cost_per_million_input_tokens(), Some(2.50));
    assert_eq!(tier.cost_per_million_output_tokens(), Some(10.0));
    assert_eq!(tier.name(), "Free");

    tracing::info!("OpenAI Free tier test passed");
    Ok(())
}

#[test]
fn test_openai_tier5() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing OpenAI Tier 5");

    let tier = OpenAITier::Tier5;

    tracing::debug!(rpm = ?tier.rpm(), tpm = ?tier.tpm(), "Checking rate limits");
    assert_eq!(tier.rpm(), Some(10_000));
    assert_eq!(tier.tpm(), Some(100_000_000));
    assert_eq!(tier.rpd(), None);
    assert_eq!(tier.max_concurrent(), Some(50));
    assert_eq!(tier.name(), "Tier 5");

    tracing::info!("OpenAI Tier 5 test passed");
    Ok(())
}
