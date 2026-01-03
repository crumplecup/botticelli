//! Tests for rate limit configuration system.

mod common;

use botticelli_error::{BotticelliResult, ConfigError};
use botticelli_interface::Tier;
use botticelli_rate_limit::{BotticelliConfig, TierConfigBuilder, TierConfigBuilderError};

#[test]
fn test_load_bundled_defaults() -> Result<(), ConfigError> {
    common::init_tracing();
    let config = BotticelliConfig::load()?;

    // Should have at least Gemini provider
    assert!(config.providers().contains_key("gemini"));

    // Gemini should have free tier
    let gemini = &config.providers()["gemini"];
    assert!(gemini.tiers().contains_key("free"));

    // Free tier should have expected limits
    let free_tier = &gemini.tiers()["free"];
    assert_eq!(free_tier.name(), "Free");
    assert_eq!(free_tier.rpm(), Some(10));
    assert_eq!(free_tier.tpm(), Some(250_000));
    assert_eq!(free_tier.rpd(), Some(250));
    Ok(())
}

#[test]
fn test_tier_config_implements_tier_trait() -> Result<(), TierConfigBuilderError> {
    common::init_tracing();
    let tier_config = TierConfigBuilder::default()
        .name("Test Tier")
        .rpm(100u32)
        .tpm(500_000u64)
        .rpd(1000u32)
        .max_concurrent(5u32)
        .daily_quota_usd(10.0)
        .cost_per_million_input_tokens(1.0)
        .cost_per_million_output_tokens(2.0)
        .build()?;

    // Test Tier trait methods
    assert_eq!(tier_config.rpm(), Some(100));
    assert_eq!(tier_config.tpm(), Some(500_000));
    assert_eq!(tier_config.rpd(), Some(1000));
    assert_eq!(tier_config.max_concurrent(), Some(5));
    assert_eq!(tier_config.daily_quota_usd(), Some(10.0));
    assert_eq!(tier_config.cost_per_million_input_tokens(), Some(1.0));
    assert_eq!(tier_config.cost_per_million_output_tokens(), Some(2.0));
    assert_eq!(tier_config.name(), "Test Tier");
    Ok(())
}

#[test]
fn test_get_tier_with_default() -> Result<(), ConfigError> {
    let config = BotticelliConfig::load()?;

    // Get default tier (should be "free" for Gemini)
    let tier = config.get_tier("gemini", None);
    assert!(tier.is_some(), "Failed to get default tier");
    let tier = tier.unwrap();

    assert_eq!(tier.name(), "Free");
    Ok(())
}

#[test]
fn test_get_tier_with_specific_name() -> Result<(), ConfigError> {
    let config = BotticelliConfig::load()?;

    // Get specific tier
    let tier = config.get_tier("gemini", Some("payasyougo"));
    assert!(tier.is_some(), "Failed to get tier");
    let tier = tier.unwrap();

    assert_eq!(tier.name(), "Pay-as-you-go");
    Ok(())
}

#[test]
fn test_config_from_file() -> BotticelliResult<()> {
    use std::io::Write;
    use tempfile::Builder;

    // Create a temporary config file with .toml extension
    let mut temp_file = Builder::new()
        .suffix(".toml")
        .tempfile()
        .map_err(botticelli_error::IoError::from)?;
    writeln!(
        temp_file,
        r#"
[providers.test]
default_tier = "custom"

[providers.test.tiers.custom]
name = "Custom Tier"
rpm = 42
tpm = 999_000
"#
    )
    .map_err(botticelli_error::IoError::from)?;

    // Load config from the temporary file
    let config = BotticelliConfig::from_file(temp_file.path())?;

    // Verify the config was loaded correctly
    assert!(config.providers().contains_key("test"));
    let tier = config.get_tier("test", Some("custom")).unwrap();
    assert_eq!(tier.name(), "Custom Tier");
    assert_eq!(tier.rpm(), Some(42));
    assert_eq!(tier.tpm(), Some(999_000));
    Ok(())
}
