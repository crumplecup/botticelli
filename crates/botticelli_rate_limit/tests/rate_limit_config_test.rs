//! Tests for rate limit configuration system.

mod helpers;

use botticelli_error::{BotticelliResult, ConfigError};
use botticelli_interface::Tier;
use botticelli_rate_limit::{BotticelliConfig, TierConfigBuilder, TierConfigBuilderError};

#[test]
fn test_load_bundled_defaults() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing bundled default config");

    let config = BotticelliConfig::load()?;

    tracing::debug!("Checking for gemini provider");
    assert!(config.providers().contains_key("gemini"));

    let gemini = &config.providers()["gemini"];
    tracing::debug!(tiers = ?gemini.tiers().keys(), "Checking gemini tiers");
    assert!(gemini.tiers().contains_key("free"));

    let free_tier = &gemini.tiers()["free"];
    tracing::debug!(name = free_tier.name(), rpm = ?free_tier.rpm(), "Validating free tier");
    assert_eq!(free_tier.name(), "Free");
    assert_eq!(free_tier.rpm(), Some(10));
    assert_eq!(free_tier.tpm(), Some(250_000));
    assert_eq!(free_tier.rpd(), Some(250));
    
    tracing::info!("Bundled defaults test passed");
    Ok(())
}

#[test]
fn test_tier_config_implements_tier_trait() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing TierConfig Tier trait implementation");

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

    tracing::debug!(name = tier_config.name(), "Validating Tier trait methods");
    assert_eq!(tier_config.rpm(), Some(100));
    assert_eq!(tier_config.tpm(), Some(500_000));
    assert_eq!(tier_config.rpd(), Some(1000));
    assert_eq!(tier_config.max_concurrent(), Some(5));
    assert_eq!(tier_config.daily_quota_usd(), Some(10.0));
    assert_eq!(tier_config.cost_per_million_input_tokens(), Some(1.0));
    assert_eq!(tier_config.cost_per_million_output_tokens(), Some(2.0));
    assert_eq!(tier_config.name(), "Test Tier");
    
    tracing::info!("TierConfig Tier trait test passed");
    Ok(())
}

#[test]
fn test_get_tier_with_default() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing get_tier with default");

    let config = BotticelliConfig::load()?;

    tracing::debug!("Getting default gemini tier");
    let tier = config.get_tier("gemini", None);
    assert!(tier.is_some(), "Failed to get default tier");
    let tier = tier.unwrap();

    tracing::debug!(tier_name = tier.name(), "Validating default tier");
    assert_eq!(tier.name(), "Free");
    
    tracing::info!("Get tier with default test passed");
    Ok(())
}

#[test]
fn test_get_tier_with_specific_name() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing get_tier with specific name");

    let config = BotticelliConfig::load()?;

    tracing::debug!("Getting specific gemini tier: payasyougo");
    let tier = config.get_tier("gemini", Some("payasyougo"));
    assert!(tier.is_some(), "Failed to get tier");
    let tier = tier.unwrap();

    tracing::debug!(tier_name = tier.name(), "Validating specific tier");
    assert_eq!(tier.name(), "Pay-as-you-go");
    
    tracing::info!("Get tier with specific name test passed");
    Ok(())
}

#[test]
fn test_config_from_file() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing config from file");

    use std::io::Write;
    use tempfile::Builder;

    tracing::debug!("Creating temporary config file");
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

    tracing::debug!(path = ?temp_file.path(), "Loading config from file");
    let config = BotticelliConfig::from_file(temp_file.path())?;

    tracing::debug!("Validating loaded config");
    assert!(config.providers().contains_key("test"));
    let tier = config.get_tier("test", Some("custom")).unwrap();
    assert_eq!(tier.name(), "Custom Tier");
    assert_eq!(tier.rpm(), Some(42));
    assert_eq!(tier.tpm(), Some(999_000));
    
    tracing::info!("Config from file test passed");
    Ok(())
}
