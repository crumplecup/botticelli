//! Configuration structures for rate limiting.
//!
//! This module provides TOML-based configuration for rate limits. The configuration
//! system supports:
//! - Bundled defaults (include_str! from botticelli.toml)
//! - User overrides (./botticelli.toml or ~/.config/botticelli/botticelli.toml)
//! - Automatic merging with user values taking precedence

use crate::Tier;
use botticelli_error::{BotticelliError, BotticelliResult, ConfigError};
use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Model-specific rate limit overrides.
///
/// These override the tier-level defaults for specific models.
/// All fields are optional - only specified fields override tier defaults.
///
/// # Example
///
/// ```toml
/// [providers.gemini.tiers.free.models."gemini-2.5-pro"]
/// rpm = 2
/// tpm = 125_000
/// rpd = 50
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct ModelTierConfig {
    /// Requests per minute limit (overrides tier default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rpm: Option<u32>,

    /// Tokens per minute limit (overrides tier default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tpm: Option<u64>,

    /// Requests per day limit (overrides tier default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rpd: Option<u32>,

    /// Maximum concurrent requests (overrides tier default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,

    /// Daily quota in USD (overrides tier default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_quota_usd: Option<f64>,

    /// Cost per million input tokens in USD (overrides tier default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_million_input_tokens: Option<f64>,

    /// Cost per million output tokens in USD (overrides tier default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_million_output_tokens: Option<f64>,
}

/// Configuration for a specific API tier.
///
/// This struct implements the `Tier` trait and can be loaded from TOML configuration.
/// All fields are optional, where `None` indicates unlimited/not applicable.
///
/// # Tier-Level Defaults
///
/// ```toml
/// [providers.gemini.tiers.free]
/// name = "Free"
/// rpm = 10
/// tpm = 250_000
/// rpd = 250
/// max_concurrent = 1
/// ```
///
/// # Model-Specific Overrides
///
/// ```toml
/// [providers.gemini.tiers.free.models."gemini-2.5-pro"]
/// rpm = 2            # Overrides tier default
/// tpm = 125_000      # Overrides tier default
/// rpd = 50           # Overrides tier default
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TierConfig {
    /// Name of the tier (e.g., "Free", "Pro", "Tier 1")
    pub name: String,

    /// Requests per minute limit (tier-level default)
    #[serde(default)]
    pub rpm: Option<u32>,

    /// Tokens per minute limit (tier-level default)
    #[serde(default)]
    pub tpm: Option<u64>,

    /// Requests per day limit (tier-level default)
    #[serde(default)]
    pub rpd: Option<u32>,

    /// Maximum concurrent requests (tier-level default)
    #[serde(default)]
    pub max_concurrent: Option<u32>,

    /// Daily quota in USD (tier-level default)
    #[serde(default)]
    pub daily_quota_usd: Option<f64>,

    /// Cost per million input tokens in USD (tier-level default)
    #[serde(default)]
    pub cost_per_million_input_tokens: Option<f64>,

    /// Cost per million output tokens in USD (tier-level default)
    #[serde(default)]
    pub cost_per_million_output_tokens: Option<f64>,

    /// Model-specific rate limit overrides
    #[serde(default)]
    pub models: HashMap<String, ModelTierConfig>,
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

impl TierConfig {
    /// Get a tier configuration with model-specific overrides applied.
    ///
    /// If the model has specific rate limit overrides in the configuration,
    /// they will override the tier-level defaults. Otherwise, returns the
    /// tier-level defaults.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model to get configuration for
    ///
    /// # Returns
    ///
    /// A new `TierConfig` with model-specific overrides applied, or a clone
    /// of the tier-level config if no model-specific overrides exist.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_rate_limit::{BotticelliConfig, Tier};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = BotticelliConfig::load()?;
    /// let tier = config.get_tier("gemini", Some("free")).unwrap();
    ///
    /// // Get config for gemini-2.5-pro (may have different limits than tier default)
    /// let model_tier = tier.for_model("gemini-2.5-pro");
    /// println!("gemini-2.5-pro RPM: {:?}", model_tier.rpm());
    /// # Ok(())
    /// # }
    /// ```
    pub fn for_model(&self, model_name: &str) -> TierConfig {
        if let Some(model_config) = self.models.get(model_name) {
            // Apply model-specific overrides
            TierConfig {
                name: self.name.clone(),
                rpm: model_config.rpm.or(self.rpm),
                tpm: model_config.tpm.or(self.tpm),
                rpd: model_config.rpd.or(self.rpd),
                max_concurrent: model_config.max_concurrent.or(self.max_concurrent),
                daily_quota_usd: model_config.daily_quota_usd.or(self.daily_quota_usd),
                cost_per_million_input_tokens: model_config
                    .cost_per_million_input_tokens
                    .or(self.cost_per_million_input_tokens),
                cost_per_million_output_tokens: model_config
                    .cost_per_million_output_tokens
                    .or(self.cost_per_million_output_tokens),
                models: HashMap::new(), // Model-specific configs don't have nested models
            }
        } else {
            // No model-specific config, return tier defaults
            self.clone()
        }
    }
}

/// Rate limit configuration for budget tracking.
///
/// Contains concrete rate limit values used by the Budget tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RateLimitConfig {
    /// Requests per minute limit
    pub requests_per_minute: u64,

    /// Tokens per minute limit
    pub tokens_per_minute: u64,

    /// Requests per day limit
    pub requests_per_day: u64,

    /// Tokens per day limit
    pub tokens_per_day: u64,
}

impl RateLimitConfig {
    /// Creates a rate limit configuration from a tier config.
    pub fn from_tier(tier: &TierConfig) -> Self {
        Self {
            requests_per_minute: tier.rpm.unwrap_or(u32::MAX) as u64,
            tokens_per_minute: tier.tpm.unwrap_or(u64::MAX),
            requests_per_day: tier.rpd.unwrap_or(u32::MAX) as u64,
            tokens_per_day: tier.tpm.unwrap_or(u64::MAX) * 1440, // Estimate: TPM * minutes per day
        }
    }
}

/// Configuration for a specific provider.
///
/// Contains the default tier name and a map of tier configurations.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// Name of the default tier for this provider
    pub default_tier: String,

    /// Map of tier name to tier configuration
    pub tiers: HashMap<String, TierConfig>,
}

/// Top-level Botticelli configuration.
///
/// Loads rate limit configurations from TOML files with a precedence system:
/// 1. Bundled defaults (include_str! from botticelli.toml)
/// 2. User override (./botticelli.toml or ~/.config/botticelli/botticelli.toml)
///
/// # Example
///
/// ```no_run
/// use botticelli_rate_limit::BotticelliConfig;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load configuration (bundled defaults + user overrides)
/// let config = BotticelliConfig::load()?;
///
/// // Get tier configuration for Gemini free tier
/// let tier = config.get_tier("gemini", Some("free")).unwrap();
/// println!("Gemini free tier RPM: {:?}", tier.rpm);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct BotticelliConfig {
    /// Map of provider name to provider configuration
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// Default budget multipliers for all providers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<botticelli_core::BudgetConfig>,

    /// Context path configuration for file references
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextConfig>,
}

/// Configuration for context file resolution.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ContextConfig {
    /// Base directory for resolving file references in narrative TOML files.
    /// Defaults to workspace root if not specified.
    pub path: Option<String>,
}

impl BotticelliConfig {
    /// Load configuration from a specific file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    #[instrument(skip(path), fields(path = %path.as_ref().display()))]
    pub fn from_file(path: impl AsRef<std::path::Path>) -> BotticelliResult<Self> {
        debug!("Loading configuration from file");

        Config::builder()
            .add_source(File::from(path.as_ref()))
            .build()
            .map_err(|e| {
                BotticelliError::from(ConfigError::new(format!(
                    "Failed to read configuration from {}: {}",
                    path.as_ref().display(),
                    e
                )))
            })?
            .try_deserialize()
            .map_err(|e| {
                BotticelliError::from(ConfigError::new(format!(
                    "Failed to parse configuration: {}",
                    e
                )))
            })
    }

    /// Load configuration with precedence: user override > bundled default.
    ///
    /// Configuration sources in order of precedence (later sources override earlier):
    /// 1. Bundled defaults (botticelli.toml shipped with library)
    /// 2. User config in home directory (~/.config/botticelli/botticelli.toml)
    /// 3. User config in current directory (./botticelli.toml)
    ///
    /// User config files are optional and will be silently skipped if not found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_rate_limit::BotticelliConfig;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = BotticelliConfig::load()?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument]
    pub fn load() -> BotticelliResult<Self> {
        debug!("Loading configuration with precedence: current dir > home dir > bundled defaults");

        // Bundled default configuration
        const DEFAULT_CONFIG: &str = include_str!("../../../botticelli.toml");

        let mut builder = Config::builder()
            // Start with bundled defaults
            .add_source(File::from_str(DEFAULT_CONFIG, FileFormat::Toml));

        // Add user config from home directory (optional)
        if let Some(home) = dirs::home_dir() {
            let home_config = home.join(".config/botticelli/botticelli.toml");
            builder = builder.add_source(File::from(home_config).required(false));
        }

        // Add user config from current directory (optional, highest precedence)
        builder = builder.add_source(File::with_name("botticelli").required(false));

        // Build and deserialize
        builder
            .build()
            .map_err(|e| {
                BotticelliError::from(ConfigError::new(format!(
                    "Failed to build configuration: {}",
                    e
                )))
            })?
            .try_deserialize()
            .map_err(|e| {
                BotticelliError::from(ConfigError::new(format!(
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
    /// use botticelli_rate_limit::BotticelliConfig;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = BotticelliConfig::load()?;
    ///
    /// // Get default tier for Gemini
    /// let tier = config.get_tier("gemini", None).unwrap();
    ///
    /// // Get specific tier
    /// let pro_tier = config.get_tier("gemini", Some("payasyougo")).unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub fn get_tier(&self, provider: &str, tier_name: Option<&str>) -> Option<TierConfig> {
        let provider_config = self.providers.get(provider)?;

        let tier = tier_name.unwrap_or(&provider_config.default_tier);

        debug!(provider, tier, "Looking up tier configuration");

        provider_config.tiers.get(tier).cloned()
    }
}
