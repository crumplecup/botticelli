//! Rate limit trait wrapper tools.
//!
//! These wrapper functions provide MCP tool access to Tier trait methods.

use crate::rmcp_server::BotticelliServer;
use botticelli_interface::Tier;
use botticelli_rate_limit::{Budget, BudgetRemaining, RateLimitConfig, TierConfig, BotticelliConfig};
#[cfg(feature = "gemini")]
use botticelli_rate_limit::GeminiTier;
#[cfg(feature = "anthropic")]
use botticelli_rate_limit::AnthropicTier;
use botticelli_rate_limit::OpenAITier;
use botticelli_error::RateLimitError;
use elicitation::Elicit;
use paste::paste;
use rmcp::tool;
use rmcp::tool_router;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::instrument;

/// Parameters for tier RPM query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierRpmParams {
    /// Tier configuration to query
    pub tier: TierConfig,
}

/// Result from tier RPM query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierRpmResult {
    /// Requests per minute limit (None = unlimited)
    pub rpm: Option<u32>,
}

/// Parameters for tier TPM query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierTpmParams {
    /// Tier configuration to query
    pub tier: TierConfig,
}

/// Result from tier TPM query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierTpmResult {
    /// Tokens per minute limit (None = unlimited)
    pub tpm: Option<u64>,
}

/// Parameters for tier RPD query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierRpdParams {
    /// Tier configuration to query
    pub tier: TierConfig,
}

/// Result from tier RPD query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierRpdResult {
    /// Requests per day limit (None = unlimited)
    pub rpd: Option<u32>,
}

/// Parameters for tier TPD query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierTpdParams {
    /// Tier configuration to query
    pub tier: TierConfig,
}

/// Result from tier TPD query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierTpdResult {
    /// Tokens per day limit (None = unlimited)
    pub tpd: Option<u64>,
}

/// Parameters for tier concurrent limit query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierMaxConcurrentParams {
    /// Tier configuration to query
    pub tier: TierConfig,
}

/// Result from tier concurrent limit query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierMaxConcurrentResult {
    /// Maximum concurrent requests (None = unlimited)
    pub max_concurrent: Option<u32>,
}

/// Parameters for tier daily quota query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierDailyQuotaParams {
    /// Tier configuration to query
    pub tier: TierConfig,
}

/// Result from tier daily quota query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierDailyQuotaResult {
    /// Daily quota in USD (None = no quota)
    pub daily_quota_usd: Option<f64>,
}

/// Parameters for tier input cost query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierInputCostParams {
    /// Tier configuration to query
    pub tier: TierConfig,
}

/// Result from tier input cost query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierInputCostResult {
    /// Cost per million input tokens in USD (None = no pricing info)
    pub cost_per_million_input_tokens: Option<f64>,
}

/// Parameters for tier output cost query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierOutputCostParams {
    /// Tier configuration to query
    pub tier: TierConfig,
}

/// Result from tier output cost query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierOutputCostResult {
    /// Cost per million output tokens in USD (None = no pricing info)
    pub cost_per_million_output_tokens: Option<f64>,
}

/// Parameters for tier name query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierNameParams {
    /// Tier configuration to query
    pub tier: TierConfig,
}

/// Result from tier name query.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct TierNameResult {
    /// Tier name
    pub name: String,
}

// ============================================================================
// Macros for generic Budget<T> wrappers
// ============================================================================

/// Macro to generate Budget wrapper methods for a specific Tier type.
macro_rules! impl_budget_wrappers {
    ($(($tier_name:ident, $tier_type:ty, $feature:literal)),* $(,)?) => {
        $(
            paste! {
                #[cfg(feature = $feature)]
                #[doc = "Create Budget<" $tier_name ">."]
                #[tool]
                #[instrument(skip(self, params))]
                pub fn [<budget_new_ $tier_name:snake>](
                    &self,
                    params: [<BudgetNew $tier_name Params>]
                ) -> Budget<$tier_type> {
                    tracing::debug!(tier = stringify!($tier_name), "Creating Budget");
                    Budget::new(params.config)
                }

                #[cfg(feature = $feature)]
                #[doc = "Reset time windows for Budget<" $tier_name ">."]
                #[tool]
                #[instrument(skip(self, params))]
                pub fn [<budget_reset_windows_ $tier_name:snake>](
                    &self,
                    mut params: [<BudgetResetWindows $tier_name Params>]
                ) -> [<BudgetResetWindows $tier_name Result>] {
                    tracing::debug!(tier = stringify!($tier_name), "Resetting budget windows");
                    params.budget.reset_windows();
                    [<BudgetResetWindows $tier_name Result>] { budget: params.budget }
                }

                #[cfg(feature = $feature)]
                #[doc = "Check if Budget<" $tier_name "> can afford tokens."]
                #[tool]
                #[instrument(skip(self, params), fields(tokens = params.tokens))]
                pub fn [<budget_can_afford_ $tier_name:snake>](
                    &self,
                    mut params: [<BudgetCanAfford $tier_name Params>]
                ) -> [<BudgetCanAfford $tier_name Result>] {
                    tracing::debug!(tier = stringify!($tier_name), tokens = params.tokens, "Checking affordability");
                    let can_afford = params.budget.can_afford(params.tokens);
                    match can_afford {
                        true => tracing::debug!("Budget can afford request"),
                        false => tracing::debug!("Budget cannot afford request"),
                    }
                    [<BudgetCanAfford $tier_name Result>] { budget: params.budget, can_afford }
                }

                #[cfg(feature = $feature)]
                #[doc = "Consume tokens in Budget<" $tier_name ">."]
                #[tool]
                #[instrument(skip(self, params), fields(tokens = params.tokens))]
                pub fn [<budget_consume_ $tier_name:snake>](
                    &self,
                    mut params: [<BudgetConsume $tier_name Params>]
                ) -> Result<[<BudgetConsume $tier_name Result>], RateLimitError> {
                    tracing::debug!(tier = stringify!($tier_name), tokens = params.tokens, "Consuming tokens");
                    
                    let result = params.budget.consume(params.tokens);
                    
                    match &result {
                        Ok(_) => tracing::debug!("Tokens consumed successfully"),
                        Err(e) => tracing::error!(error = ?e, "Failed to consume tokens"),
                    }
                    
                    result.map(|_| [<BudgetConsume $tier_name Result>] { budget: params.budget })
                }

                #[cfg(feature = $feature)]
                #[doc = "Get remaining budget for Budget<" $tier_name ">."]
                #[tool]
                #[instrument(skip(self, params))]
                pub fn [<budget_remaining_ $tier_name:snake>](
                    &self,
                    mut params: [<BudgetRemaining $tier_name Params>]
                ) -> [<BudgetRemaining $tier_name Result>] {
                    tracing::debug!(tier = stringify!($tier_name), "Getting remaining budget");
                    let remaining = params.budget.remaining();
                    tracing::debug!(
                        tokens_per_minute = remaining.tokens_per_minute(),
                        tokens_per_day = remaining.tokens_per_day(),
                        "Remaining budget calculated"
                    );
                    [<BudgetRemaining $tier_name Result>] { budget: params.budget, remaining }
                }
            }
        )*
    };
}

/// Macro to generate DTOs for Budget wrappers.
macro_rules! impl_budget_dtos {
    ($(($tier_name:ident, $tier_type:ty, $feature:literal)),* $(,)?) => {
        $(
            paste! {
                #[cfg(feature = $feature)]
                #[doc = "Parameters for creating Budget<" $tier_name ">."]
                #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
                pub struct [<BudgetNew $tier_name Params>] {
                    /// Tier configuration
                    pub config: $tier_type,
                }

                #[cfg(feature = $feature)]
                #[doc = "Parameters for resetting Budget<" $tier_name "> windows."]
                #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
                pub struct [<BudgetResetWindows $tier_name Params>] {
                    /// Budget to reset
                    pub budget: Budget<$tier_type>,
                }

                #[cfg(feature = $feature)]
                #[doc = "Result of resetting Budget<" $tier_name "> windows."]
                #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
                pub struct [<BudgetResetWindows $tier_name Result>] {
                    /// Updated budget
                    pub budget: Budget<$tier_type>,
                }

                #[cfg(feature = $feature)]
                #[doc = "Parameters for checking Budget<" $tier_name "> affordability."]
                #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
                pub struct [<BudgetCanAfford $tier_name Params>] {
                    /// Budget to check
                    pub budget: Budget<$tier_type>,
                    /// Number of tokens requested
                    pub tokens: u64,
                }

                #[cfg(feature = $feature)]
                #[doc = "Result of checking Budget<" $tier_name "> affordability."]
                #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
                pub struct [<BudgetCanAfford $tier_name Result>] {
                    /// Updated budget
                    pub budget: Budget<$tier_type>,
                    /// Whether the budget can afford the request
                    pub can_afford: bool,
                }

                #[cfg(feature = $feature)]
                #[doc = "Parameters for consuming tokens in Budget<" $tier_name ">."]
                #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
                pub struct [<BudgetConsume $tier_name Params>] {
                    /// Budget to consume from
                    pub budget: Budget<$tier_type>,
                    /// Number of tokens to consume
                    pub tokens: u64,
                }

                #[cfg(feature = $feature)]
                #[doc = "Result of consuming tokens in Budget<" $tier_name ">."]
                #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
                pub struct [<BudgetConsume $tier_name Result>] {
                    /// Updated budget
                    pub budget: Budget<$tier_type>,
                }

                #[cfg(feature = $feature)]
                #[doc = "Parameters for getting remaining Budget<" $tier_name ">."]
                #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
                pub struct [<BudgetRemaining $tier_name Params>] {
                    /// Budget to query
                    pub budget: Budget<$tier_type>,
                }

                #[cfg(feature = $feature)]
                #[doc = "Result of getting remaining Budget<" $tier_name ">."]
                #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
                pub struct [<BudgetRemaining $tier_name Result>] {
                    /// Updated budget
                    pub budget: Budget<$tier_type>,
                    /// Remaining budget across windows
                    pub remaining: BudgetRemaining,
                }
            }
        )*
    };
}

// Generate DTOs for all tier types
impl_budget_dtos! {
    (TierConfig, TierConfig, "default"),
    (GeminiTier, GeminiTier, "gemini"),
    (AnthropicTier, AnthropicTier, "anthropic"),
    (OpenAITier, OpenAITier, "default"),
}

#[tool_router(router = rate_limit_tool_router, vis = "pub")]
impl BotticelliServer {
    /// Get requests per minute limit for a tier.
    #[tool]
    #[instrument(skip(self))]
    pub fn tier_rpm(&self, params: TierRpmParams) -> TierRpmResult {
        let rpm = Tier::rpm(&params.tier);
        tracing::info!(rpm = ?rpm, "Retrieved RPM limit");
        TierRpmResult { rpm }
    }

    /// Get tokens per minute limit for a tier.
    #[tool]
    #[instrument(skip(self))]
    pub fn tier_tpm(&self, params: TierTpmParams) -> TierTpmResult {
        let tpm = Tier::tpm(&params.tier);
        tracing::info!(tpm = ?tpm, "Retrieved TPM limit");
        TierTpmResult { tpm }
    }

    /// Get requests per day limit for a tier.
    #[tool]
    #[instrument(skip(self))]
    pub fn tier_rpd(&self, params: TierRpdParams) -> TierRpdResult {
        let rpd = Tier::rpd(&params.tier);
        tracing::info!(rpd = ?rpd, "Retrieved RPD limit");
        TierRpdResult { rpd }
    }

    /// Get tokens per day limit for a tier.
    #[tool]
    #[instrument(skip(self))]
    pub fn tier_tpd(&self, params: TierTpdParams) -> TierTpdResult {
        let tpd = Tier::tpd(&params.tier);
        tracing::info!(tpd = ?tpd, "Retrieved TPD limit");
        TierTpdResult { tpd }
    }

    /// Get maximum concurrent requests limit for a tier.
    #[tool]
    #[instrument(skip(self))]
    pub fn tier_max_concurrent(&self, params: TierMaxConcurrentParams) -> TierMaxConcurrentResult {
        let max_concurrent = Tier::max_concurrent(&params.tier);
        tracing::info!(max_concurrent = ?max_concurrent, "Retrieved max concurrent limit");
        TierMaxConcurrentResult { max_concurrent }
    }

    /// Get daily quota in USD for a tier.
    #[tool]
    #[instrument(skip(self))]
    pub fn tier_daily_quota_usd(&self, params: TierDailyQuotaParams) -> TierDailyQuotaResult {
        let daily_quota_usd = Tier::daily_quota_usd(&params.tier);
        tracing::info!(daily_quota_usd = ?daily_quota_usd, "Retrieved daily quota");
        TierDailyQuotaResult { daily_quota_usd }
    }

    /// Get cost per million input tokens for a tier.
    #[tool]
    #[instrument(skip(self))]
    pub fn tier_cost_per_million_input_tokens(&self, params: TierInputCostParams) -> TierInputCostResult {
        let cost = Tier::cost_per_million_input_tokens(&params.tier);
        tracing::info!(cost = ?cost, "Retrieved input token cost");
        TierInputCostResult {
            cost_per_million_input_tokens: cost,
        }
    }

    /// Get cost per million output tokens for a tier.
    #[tool]
    #[instrument(skip(self))]
    pub fn tier_cost_per_million_output_tokens(&self, params: TierOutputCostParams) -> TierOutputCostResult {
        let cost = Tier::cost_per_million_output_tokens(&params.tier);
        tracing::info!(cost = ?cost, "Retrieved output token cost");
        TierOutputCostResult {
            cost_per_million_output_tokens: cost,
        }
    }

    /// Get tier name.
    #[tool]
    #[instrument(skip(self))]
    pub fn tier_name(&self, params: TierNameParams) -> TierNameResult {
        let name = Tier::name(&params.tier).to_string();
        tracing::info!(name = %name, "Retrieved tier name");
        TierNameResult { name }
    }

    /// Get tier configuration for a specific model.
    #[tool]
    #[instrument(skip(self, params), fields(model = %params.model_name))]
    pub fn rate_limit_for_model(&self, params: RateLimitForModelParams) -> TierConfig {
        tracing::debug!("Delegating to TierConfig::for_model");
        let result = params.tier_config.for_model(&params.model_name);
        tracing::debug!(tier_name = %result.name(), "Model tier retrieved");
        result
    }

    /// Create rate limit config from tier configuration.
    #[tool]
    #[instrument(skip(self, params), fields(tier_name = %params.tier.name()))]
    pub fn rate_limit_from_tier(&self, params: RateLimitFromTierParams) -> RateLimitConfig {
        tracing::debug!("Delegating to RateLimitConfig::from_tier");
        let config = RateLimitConfig::from_tier(&params.tier);
        tracing::debug!("Rate limit config created from tier");
        config
    }

    /// Create unlimited rate limit config.
    #[tool]
    #[instrument(skip(self, params), fields(name = %params.name))]
    pub fn rate_limit_unlimited(&self, params: RateLimitUnlimitedParams) -> RateLimitConfig {
        tracing::debug!("Delegating to RateLimitConfig::unlimited");
        let config = RateLimitConfig::unlimited(&params.name);
        tracing::debug!("Unlimited rate limit config created");
        config
    }

    /// Load botticelli config from file.
    #[tool]
    #[instrument(skip(self, params), fields(path = ?params.path))]
    pub fn rate_limit_from_file(&self, params: RateLimitFromFileParams) -> Result<BotticelliConfig, botticelli_error::ConfigError> {
        tracing::debug!("Delegating to BotticelliConfig::from_file");
        
        let result = BotticelliConfig::from_file(&params.path);
        
        match &result {
            Ok(_) => tracing::debug!("Config loaded from file"),
            Err(e) => tracing::error!(error = ?e, "Config load failed"),
        }
        
        result
    }

    /// Load botticelli config from default location.
    #[tool]
    #[instrument(skip(self), fields(tool = "rate_limit_load"))]
    pub fn rate_limit_load(&self) -> Result<BotticelliConfig, botticelli_error::ConfigError> {
        tracing::debug!("Delegating to BotticelliConfig::load");
        
        let result = BotticelliConfig::load();
        
        match &result {
            Ok(_) => tracing::debug!("Config loaded from default location"),
            Err(e) => tracing::error!(error = ?e, "Config load failed"),
        }
        
        result
    }

    /// Get tier configuration from loaded config.
    #[tool]
    #[instrument(skip(self, params), fields(provider = %params.provider))]
    pub fn rate_limit_get_tier(&self, params: RateLimitGetTierParams) -> RateLimitGetTierResult {
        tracing::debug!("Delegating to BotticelliConfig::get_tier");
        
        let tier = params.config.get_tier(&params.provider, params.tier_name.as_deref());
        
        match &tier {
            Some(t) => tracing::debug!(tier_name = %t.name(), "Tier retrieved"),
            None => tracing::debug!("Tier not found"),
        }
        
        RateLimitGetTierResult { tier }
    }

    // Generate Budget<T> wrappers for all tier types
    impl_budget_wrappers! {
        (TierConfig, TierConfig, "default"),
        (GeminiTier, GeminiTier, "gemini"),
        (AnthropicTier, AnthropicTier, "anthropic"),
        (OpenAITier, OpenAITier, "default"),
    }
}

// ============================================================================
// DTOs for new primitives
// ============================================================================

/// Parameters for getting model tier.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct RateLimitForModelParams {
    /// Tier configuration
    pub tier_config: TierConfig,
    /// Model name
    pub model_name: String,
}

/// Parameters for creating config from tier.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct RateLimitFromTierParams {
    /// Tier configuration
    pub tier: TierConfig,
}

/// Parameters for creating unlimited config.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct RateLimitUnlimitedParams {
    /// Config name
    pub name: String,
}

/// Parameters for loading config from file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct RateLimitFromFileParams {
    /// Path to config file
    pub path: PathBuf,
}

/// Parameters for getting tier from config.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct RateLimitGetTierParams {
    /// Botticelli configuration
    pub config: BotticelliConfig,
    /// Provider name
    pub provider: String,
    /// Optional tier name
    pub tier_name: Option<String>,
}

/// Result of getting tier.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct RateLimitGetTierResult {
    /// Tier configuration if found
    pub tier: Option<TierConfig>,
}
