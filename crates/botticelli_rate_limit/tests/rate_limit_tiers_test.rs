//! Tests for rate limit tiers.

use botticelli_rate_limit::Tier;

#[cfg(feature = "gemini")]
use botticelli_rate_limit::GeminiTier;

#[cfg(feature = "anthropic")]
use botticelli_rate_limit::AnthropicTier;

use botticelli_rate_limit::tiers::OpenAITier;

#[cfg(feature = "gemini")]
#[test]
fn test_gemini_free_tier() {
    let tier = GeminiTier::Free;
    assert_eq!(tier.rpm(), Some(10));
    assert_eq!(tier.tpm(), Some(250_000));
    assert_eq!(tier.rpd(), Some(250));
    assert_eq!(tier.max_concurrent(), Some(1));
    assert_eq!(tier.cost_per_million_input_tokens(), Some(0.0));
    assert_eq!(tier.cost_per_million_output_tokens(), Some(0.0));
    assert_eq!(tier.daily_quota_usd(), None);
    assert_eq!(tier.name(), "Free");
}

#[cfg(feature = "gemini")]
#[test]
fn test_gemini_payasyougo_tier() {
    let tier = GeminiTier::PayAsYouGo;
    assert_eq!(tier.rpm(), Some(360));
    assert_eq!(tier.tpm(), Some(4_000_000));
    assert_eq!(tier.rpd(), None);
    assert_eq!(tier.max_concurrent(), Some(1));
    assert_eq!(tier.cost_per_million_input_tokens(), Some(0.075));
    assert_eq!(tier.cost_per_million_output_tokens(), Some(0.30));
    assert_eq!(tier.name(), "Pay-as-you-go");
}

#[cfg(feature = "anthropic")]
#[test]
fn test_anthropic_tier1() {
    let tier = AnthropicTier::Tier1;
    assert_eq!(tier.rpm(), Some(5));
    assert_eq!(tier.tpm(), Some(20_000));
    assert_eq!(tier.rpd(), None);
    assert_eq!(tier.max_concurrent(), Some(5));
    assert_eq!(tier.cost_per_million_input_tokens(), Some(3.0));
    assert_eq!(tier.cost_per_million_output_tokens(), Some(15.0));
    assert_eq!(tier.name(), "Tier 1");
}

#[cfg(feature = "anthropic")]
#[test]
fn test_anthropic_tier4() {
    let tier = AnthropicTier::Tier4;
    assert_eq!(tier.rpm(), Some(2000));
    assert_eq!(tier.tpm(), Some(160_000));
    assert_eq!(tier.max_concurrent(), Some(5));
    assert_eq!(tier.name(), "Tier 4");
}

#[test]
fn test_openai_free_tier() {
    let tier = OpenAITier::Free;
    assert_eq!(tier.rpm(), Some(3));
    assert_eq!(tier.tpm(), Some(40_000));
    assert_eq!(tier.rpd(), Some(200));
    assert_eq!(tier.max_concurrent(), Some(50));
    assert_eq!(tier.cost_per_million_input_tokens(), Some(2.50));
    assert_eq!(tier.cost_per_million_output_tokens(), Some(10.0));
    assert_eq!(tier.name(), "Free");
}

#[test]
fn test_openai_tier5() {
    let tier = OpenAITier::Tier5;
    assert_eq!(tier.rpm(), Some(10_000));
    assert_eq!(tier.tpm(), Some(100_000_000));
    assert_eq!(tier.rpd(), None);
    assert_eq!(tier.max_concurrent(), Some(50));
    assert_eq!(tier.name(), "Tier 5");
}
