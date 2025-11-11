//! Configuration structures for rate limiting.
//!
//! This module provides TOML-based configuration for rate limits. The configuration
//! system supports:
//! - Bundled defaults (include_str! from boticelli.toml)
//! - User overrides (./boticelli.toml or ~/.config/boticelli/boticelli.toml)
//! - Automatic merging with user values taking precedence

use crate::rate_limit::Tier;
use crate::{BoticelliError, BoticelliErrorKind, BoticelliResult};
use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a specific API tier.
///
/// This struct implements the `Tier` trait and can be loaded from TOML configuration.
/// All fields are optional, where `None` indicates unlimited/not applicable.
///
/// # Example
///
/// ```toml
/// [providers.gemini.tiers.free]
/// name = "Free"
/// rpm = 10
/// tpm = 250_000
/// rpd = 250
/// max_concurrent = 1
/// cost_per_million_input_tokens = 0.0
/// cost_per_million_output_tokens = 0.0
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TierConfig {
    /// Name of the tier (e.g., "Free", "Pro", "Tier 1")
    pub name: String,

    /// Requests per minute limit
    #[serde(default)]
    pub rpm: Option<u32>,

    /// Tokens per minute limit
    #[serde(default)]
    pub tpm: Option<u64>,

    /// Requests per day limit
    #[serde(default)]
    pub rpd: Option<u32>,

    /// Maximum concurrent requests
    #[serde(default)]
    pub max_concurrent: Option<u32>,

    /// Daily quota in USD
    #[serde(default)]
    pub daily_quota_usd: Option<f64>,

    /// Cost per million input tokens in USD
    #[serde(default)]
    pub cost_per_million_input_tokens: Option<f64>,

    /// Cost per million output tokens in USD
    #[serde(default)]
    pub cost_per_million_output_tokens: Option<f64>,
}

impl Tier for TierConfig {
    fn rpm(&self) -> Option<u32> {
        self.rpm
    }

    fn tpm(&self) -> Option<u64> {
        self.tpm
    }

    fn rpd(&self) -> Option<u32> {
        self.rpd
    }

    fn max_concurrent(&self) -> Option<u32> {
        self.max_concurrent
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        self.daily_quota_usd
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        self.cost_per_million_input_tokens
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        self.cost_per_million_output_tokens
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Configuration for a specific provider.
///
/// Contains the default tier name and a map of tier configurations.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// Name of the default tier for this provider
    pub default_tier: String,

    /// Map of tier name to tier configuration
    pub tiers: HashMap<String, TierConfig>,
}

/// Top-level Boticelli configuration.
///
/// Loads rate limit configurations from TOML files with a precedence system:
/// 1. Bundled defaults (include_str! from boticelli.toml)
/// 2. User override (./boticelli.toml or ~/.config/boticelli/boticelli.toml)
///
/// # Example
///
/// ```no_run
/// use boticelli::BoticelliConfig;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load configuration (bundled defaults + user overrides)
/// let config = BoticelliConfig::load()?;
///
/// // Get tier configuration for Gemini free tier
/// let tier = config.get_tier("gemini", Some("free")).unwrap();
/// println!("Gemini free tier RPM: {:?}", tier.rpm);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BoticelliConfig {
    /// Map of provider name to provider configuration
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

impl BoticelliConfig {
    /// Load configuration from a specific file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> BoticelliResult<Self> {
        Config::builder()
            .add_source(File::from(path.as_ref()))
            .build()
            .map_err(|e| {
                BoticelliError::new(BoticelliErrorKind::Config(format!(
                    "Failed to read configuration from {}: {}",
                    path.as_ref().display(),
                    e
                )))
            })?
            .try_deserialize()
            .map_err(|e| {
                BoticelliError::new(BoticelliErrorKind::Config(format!(
                    "Failed to parse configuration: {}",
                    e
                )))
            })
    }

    /// Load configuration with precedence: user override > bundled default.
    ///
    /// Configuration sources in order of precedence (later sources override earlier):
    /// 1. Bundled defaults (boticelli.toml shipped with library)
    /// 2. User config in home directory (~/.config/boticelli/boticelli.toml)
    /// 3. User config in current directory (./boticelli.toml)
    ///
    /// User config files are optional and will be silently skipped if not found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use boticelli::BoticelliConfig;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = BoticelliConfig::load()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load() -> BoticelliResult<Self> {
        // Bundled default configuration
        const DEFAULT_CONFIG: &str = include_str!("../../boticelli.toml");

        let mut builder = Config::builder()
            // Start with bundled defaults
            .add_source(File::from_str(DEFAULT_CONFIG, FileFormat::Toml));

        // Add user config from home directory (optional)
        if let Some(home) = dirs::home_dir() {
            let home_config = home.join(".config/boticelli/boticelli.toml");
            builder = builder.add_source(File::from(home_config).required(false));
        }

        // Add user config from current directory (optional, highest precedence)
        builder = builder.add_source(File::with_name("boticelli").required(false));

        // Build and deserialize
        builder
            .build()
            .map_err(|e| {
                BoticelliError::new(BoticelliErrorKind::Config(format!(
                    "Failed to build configuration: {}",
                    e
                )))
            })?
            .try_deserialize()
            .map_err(|e| {
                BoticelliError::new(BoticelliErrorKind::Config(format!(
                    "Failed to parse configuration: {}",
                    e
                )))
            })
    }

    /// Get tier configuration for a provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider name (e.g., "gemini", "anthropic", "openai")
    /// * `tier_name` - Optional tier name (uses provider's default if None)
    ///
    /// # Returns
    ///
    /// Returns `Some(TierConfig)` if found, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use boticelli::BoticelliConfig;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = BoticelliConfig::load()?;
    ///
    /// // Get default tier for Gemini
    /// let tier = config.get_tier("gemini", None).unwrap();
    ///
    /// // Get specific tier
    /// let pro_tier = config.get_tier("gemini", Some("payasyougo")).unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_tier(&self, provider: &str, tier_name: Option<&str>) -> Option<TierConfig> {
        let provider_config = self.providers.get(provider)?;

        let tier = tier_name.unwrap_or(&provider_config.default_tier);

        provider_config.tiers.get(tier).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_bundled_defaults() {
        let config = BoticelliConfig::load().unwrap();

        // Should have at least Gemini provider
        assert!(config.providers.contains_key("gemini"));

        // Gemini should have free tier
        let gemini = &config.providers["gemini"];
        assert!(gemini.tiers.contains_key("free"));

        // Free tier should have expected limits
        let free_tier = &gemini.tiers["free"];
        assert_eq!(free_tier.name, "Free");
        assert_eq!(free_tier.rpm, Some(10));
        assert_eq!(free_tier.tpm, Some(250_000));
        assert_eq!(free_tier.rpd, Some(250));
    }

    #[test]
    fn test_tier_config_implements_tier_trait() {
        let tier_config = TierConfig {
            name: "Test Tier".to_string(),
            rpm: Some(100),
            tpm: Some(500_000),
            rpd: Some(1000),
            max_concurrent: Some(5),
            daily_quota_usd: Some(10.0),
            cost_per_million_input_tokens: Some(1.0),
            cost_per_million_output_tokens: Some(2.0),
        };

        // Test Tier trait methods
        assert_eq!(tier_config.rpm(), Some(100));
        assert_eq!(tier_config.tpm(), Some(500_000));
        assert_eq!(tier_config.rpd(), Some(1000));
        assert_eq!(tier_config.max_concurrent(), Some(5));
        assert_eq!(tier_config.daily_quota_usd(), Some(10.0));
        assert_eq!(tier_config.cost_per_million_input_tokens(), Some(1.0));
        assert_eq!(tier_config.cost_per_million_output_tokens(), Some(2.0));
        assert_eq!(tier_config.name(), "Test Tier");
    }

    #[test]
    fn test_get_tier_with_default() {
        let config = BoticelliConfig::load().unwrap();

        // Get default tier (should be "free" for Gemini)
        let tier = config.get_tier("gemini", None).unwrap();
        assert_eq!(tier.name, "Free");
    }

    #[test]
    fn test_get_tier_with_specific_name() {
        let config = BoticelliConfig::load().unwrap();

        // Get specific tier
        let tier = config.get_tier("gemini", Some("payasyougo")).unwrap();
        assert_eq!(tier.name, "Pay-as-you-go");
    }

    #[test]
    fn test_config_from_file() {
        use std::io::Write;
        use tempfile::Builder;

        // Create a temporary config file with .toml extension
        let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
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
        .unwrap();

        // Load config from the temporary file
        let config = BoticelliConfig::from_file(temp_file.path()).unwrap();

        // Verify the config was loaded correctly
        assert!(config.providers.contains_key("test"));
        let tier = config.get_tier("test", Some("custom")).unwrap();
        assert_eq!(tier.name, "Custom Tier");
        assert_eq!(tier.rpm, Some(42));
        assert_eq!(tier.tpm, Some(999_000));
    }
}
