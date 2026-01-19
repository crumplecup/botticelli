//! Budget configuration for rate limiting multipliers.

use botticelli_error::ConfigError;
use rmcp::tool;
use serde::{Deserialize, Serialize};

/// Budget multipliers for throttling API usage.
///
/// Multipliers scale the effective rate limits without modifying tier configuration.
/// All multipliers are in the range (0.0, 1.0] where 1.0 means full quota usage.
///
/// # Examples
///
/// ```
/// use botticelli_core::BudgetConfig;
///
/// // Use 80% of RPM, 50% of RPD
/// let conservative = BudgetConfig::builder()
///     .rpm_multiplier(0.8)
///     .rpd_multiplier(0.5)
///     .build();
///
/// // Default: use full quotas
/// let full = BudgetConfig::default();
/// assert_eq!(*full.rpm_multiplier(), 1.0);
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_builder::Builder,
    elicitation::Elicit,
)]
#[serde(deny_unknown_fields)]
#[builder(pattern = "owned", setter(into, strip_option))]
pub struct BudgetConfig {
    /// Multiplier for requests per minute (0.0-1.0, default 1.0).
    #[serde(default = "default_multiplier")]
    #[builder(default = "1.0")]
    rpm_multiplier: f64,

    /// Multiplier for tokens per minute (0.0-1.0, default 1.0).
    #[serde(default = "default_multiplier")]
    #[builder(default = "1.0")]
    tpm_multiplier: f64,

    /// Multiplier for requests per day (0.0-1.0, default 1.0).
    #[serde(default = "default_multiplier")]
    #[builder(default = "1.0")]
    rpd_multiplier: f64,
}

fn default_multiplier() -> f64 {
    1.0
}

#[tool]
fn default_multiplier_tool() -> f64 {
    default_multiplier()
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            rpm_multiplier: 1.0,
            tpm_multiplier: 1.0,
            rpd_multiplier: 1.0,
        }
    }
}

impl BudgetConfig {
    /// Creates a new budget config builder.
    #[tool]
    #[tracing::instrument]
    pub fn builder() -> BudgetConfigBuilder {
        BudgetConfigBuilder::default()
    }

    /// Validates that all multipliers are in valid range (0.0, 1.0].
    ///
    /// # Errors
    ///
    /// Returns an error if any multiplier is <= 0.0 or > 1.0.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.rpm_multiplier <= 0.0 || self.rpm_multiplier > 1.0 {
            return Err(ConfigError::new(format!(
                "RPM multiplier must be in (0.0, 1.0], got {}",
                self.rpm_multiplier
            )));
        }
        if self.tpm_multiplier <= 0.0 || self.tpm_multiplier > 1.0 {
            return Err(ConfigError::new(format!(
                "TPM multiplier must be in (0.0, 1.0], got {}",
                self.tpm_multiplier
            )));
        }
        if self.rpd_multiplier <= 0.0 || self.rpd_multiplier > 1.0 {
            return Err(ConfigError::new(format!(
                "RPD multiplier must be in (0.0, 1.0], got {}",
                self.rpd_multiplier
            )));
        }
        Ok(())
    }

    /// Applies this budget to a rate limit value.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn apply_rpm(&self, rpm: u64) -> u64 {
        (rpm as f64 * self.rpm_multiplier).round() as u64
    }

    /// Applies this budget to a token limit value.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn apply_tpm(&self, tpm: u64) -> u64 {
        (tpm as f64 * self.tpm_multiplier).round() as u64
    }

    /// Applies this budget to a daily request limit.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn apply_rpd(&self, rpd: u64) -> u64 {
        (rpd as f64 * self.rpd_multiplier).round() as u64
    }

    /// Merges this budget with another, taking the minimum of each multiplier.
    ///
    /// This is useful for combining CLI overrides with narrative config.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn merge(&self, other: &BudgetConfig) -> BudgetConfig {
        BudgetConfig {
            rpm_multiplier: self.rpm_multiplier.min(other.rpm_multiplier),
            tpm_multiplier: self.tpm_multiplier.min(other.tpm_multiplier),
            rpd_multiplier: self.rpd_multiplier.min(other.rpd_multiplier),
        }
    }
}
