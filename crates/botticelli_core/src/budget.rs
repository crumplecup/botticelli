//! Budget configuration for rate limiting multipliers.

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, derive_getters::Getters)]
#[serde(deny_unknown_fields)]
pub struct BudgetConfig {
    /// Multiplier for requests per minute (0.0-1.0, default 1.0).
    #[serde(default = "default_multiplier")]
    rpm_multiplier: f64,

    /// Multiplier for tokens per minute (0.0-1.0, default 1.0).
    #[serde(default = "default_multiplier")]
    tpm_multiplier: f64,

    /// Multiplier for requests per day (0.0-1.0, default 1.0).
    #[serde(default = "default_multiplier")]
    rpd_multiplier: f64,
}

fn default_multiplier() -> f64 {
    1.0
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
    pub fn builder() -> BudgetConfigBuilder {
        BudgetConfigBuilder::default()
    }

    /// Validates that all multipliers are in valid range (0.0, 1.0].
    ///
    /// # Errors
    ///
    /// Returns an error if any multiplier is <= 0.0 or > 1.0.
    pub fn validate(&self) -> Result<(), String> {
        if self.rpm_multiplier <= 0.0 || self.rpm_multiplier > 1.0 {
            return Err(format!(
                "RPM multiplier must be in (0.0, 1.0], got {}",
                self.rpm_multiplier
            ));
        }
        if self.tpm_multiplier <= 0.0 || self.tpm_multiplier > 1.0 {
            return Err(format!(
                "TPM multiplier must be in (0.0, 1.0], got {}",
                self.tpm_multiplier
            ));
        }
        if self.rpd_multiplier <= 0.0 || self.rpd_multiplier > 1.0 {
            return Err(format!(
                "RPD multiplier must be in (0.0, 1.0], got {}",
                self.rpd_multiplier
            ));
        }
        Ok(())
    }

    /// Applies this budget to a rate limit value.
    pub fn apply_rpm(&self, rpm: u64) -> u64 {
        (rpm as f64 * self.rpm_multiplier).round() as u64
    }

    /// Applies this budget to a token limit value.
    pub fn apply_tpm(&self, tpm: u64) -> u64 {
        (tpm as f64 * self.tpm_multiplier).round() as u64
    }

    /// Applies this budget to a daily request limit.
    pub fn apply_rpd(&self, rpd: u64) -> u64 {
        (rpd as f64 * self.rpd_multiplier).round() as u64
    }

    /// Merges this budget with another, taking the minimum of each multiplier.
    ///
    /// This is useful for combining CLI overrides with narrative config.
    pub fn merge(&self, other: &BudgetConfig) -> BudgetConfig {
        BudgetConfig {
            rpm_multiplier: self.rpm_multiplier.min(other.rpm_multiplier),
            tpm_multiplier: self.tpm_multiplier.min(other.tpm_multiplier),
            rpd_multiplier: self.rpd_multiplier.min(other.rpd_multiplier),
        }
    }
}

/// Builder for `BudgetConfig`.
#[derive(Debug, Default)]
pub struct BudgetConfigBuilder {
    rpm_multiplier: Option<f64>,
    tpm_multiplier: Option<f64>,
    rpd_multiplier: Option<f64>,
}

impl BudgetConfigBuilder {
    /// Sets the RPM multiplier.
    pub fn rpm_multiplier(mut self, value: f64) -> Self {
        self.rpm_multiplier = Some(value);
        self
    }

    /// Sets the TPM multiplier.
    pub fn tpm_multiplier(mut self, value: f64) -> Self {
        self.tpm_multiplier = Some(value);
        self
    }

    /// Sets the RPD multiplier.
    pub fn rpd_multiplier(mut self, value: f64) -> Self {
        self.rpd_multiplier = Some(value);
        self
    }

    /// Builds the `BudgetConfig`.
    pub fn build(self) -> BudgetConfig {
        BudgetConfig {
            rpm_multiplier: self.rpm_multiplier.unwrap_or(1.0),
            tpm_multiplier: self.tpm_multiplier.unwrap_or(1.0),
            rpd_multiplier: self.rpd_multiplier.unwrap_or(1.0),
        }
    }
}


