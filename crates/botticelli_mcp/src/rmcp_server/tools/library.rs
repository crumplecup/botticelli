//! Library function tools - expose botticelli workspace functions as MCP tools.
//!
//! This module demonstrates the "universal tooling" strategy: wrapping library
//! functions as MCP tools so LLMs can call them directly and compose workflows.

use crate::rmcp_server::helpers::to_mcp_error;
use crate::rmcp_server::BotticelliServer;
use botticelli_narrative::validator::{ValidationConfig, Validator};
use botticelli_rate_limit::BotticelliConfig;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::ErrorCode;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use tracing::{debug, instrument};

/// Parameters for validating narrative TOML.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct ValidateTomlParams {
    /// The TOML content to validate
    toml: String,
    
    /// Whether to check for warnings (default: true)
    #[serde(default = "default_check_warnings")]
    check_warnings: bool,
}

fn default_check_warnings() -> bool {
    true
}

/// Result from TOML validation.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct ValidateTomlResult {
    /// Whether the TOML is valid
    valid: bool,
    
    /// List of error messages
    errors: Vec<String>,
    
    /// List of warning messages  
    warnings: Vec<String>,
}

/// Parameters for getting rate limit tier information.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct GetTierInfoParams {
    /// Provider name (e.g., "gemini", "anthropic", "openai")
    provider: String,
    
    /// Tier name (e.g., "free", "pro", "tier1")
    #[serde(skip_serializing_if = "Option::is_none")]
    tier: Option<String>,
}

/// Tier information result.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct TierInfo {
    /// Tier name
    name: String,
    
    /// Requests per minute limit
    #[serde(skip_serializing_if = "Option::is_none")]
    rpm: Option<u32>,
    
    /// Tokens per minute limit
    #[serde(skip_serializing_if = "Option::is_none")]
    tpm: Option<u64>,
    
    /// Requests per day limit
    #[serde(skip_serializing_if = "Option::is_none")]
    rpd: Option<u32>,
}

/// Result from getting tier information.
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct GetTierInfoResult {
    /// Provider name
    provider: String,
    
    /// Tier information
    tier: TierInfo,
}

impl BotticelliServer {
    /// Validate narrative TOML without creating a file.
    ///
    /// Exposes `botticelli_narrative::Validator::validate_toml` as an MCP tool.
    #[instrument(skip(self, params), fields(toml_len = params.toml().len()))]
    pub async fn validate_toml(
        &self,
        Parameters(params): Parameters<ValidateTomlParams>,
    ) -> Result<Json<ValidateTomlResult>, rmcp::ErrorData> {
        let toml = params.toml();
        debug!(toml_len = toml.len(), "Validating TOML");

        // Call library function
        let validation = Validator::validate_toml(toml);

        let errors: Vec<String> = validation
            .errors()
            .iter()
            .map(|e| e.message().to_string())
            .collect();

        let warnings: Vec<String> = if *params.check_warnings() {
            validation
                .warnings()
                .iter()
                .map(|w| w.message().to_string())
                .collect()
        } else {
            Vec::new()
        };

        let result = ValidateTomlResult {
            valid: validation.is_valid(),
            errors,
            warnings,
        };

        debug!(
            valid = result.valid,
            error_count = result.errors.len(),
            warning_count = result.warnings.len(),
            "TOML validation complete"
        );

        Ok(Json(result))
    }

    /// Get rate limit tier information for a provider.
    ///
    /// Exposes `botticelli_rate_limit::BotticelliConfig` tier lookup as an MCP tool.
    #[instrument(skip(self, params), fields(provider = %params.provider()))]
    pub async fn get_tier_info(
        &self,
        Parameters(params): Parameters<GetTierInfoParams>,
    ) -> Result<Json<GetTierInfoResult>, rmcp::ErrorData> {
        let provider = params.provider();
        let tier_name = params.tier();
        
        debug!(provider, tier = ?tier_name, "Getting tier information");

        // Load rate limit configuration
        let config = BotticelliConfig::load()
            .map_err(|e| to_mcp_error(e, "Failed to load rate limit config"))?;

        // Get tier configuration
        let tier = config
            .get_tier(provider, tier_name.as_deref())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    Cow::Owned(format!(
                        "Tier not found for provider '{}' with tier '{}'",
                        provider,
                        tier_name.as_deref().unwrap_or("default")
                    )),
                    None,
                )
            })?;

        let tier_info = TierInfo {
            name: tier.name().to_string(),
            rpm: tier.rpm().clone(),
            tpm: tier.tpm().clone(),
            rpd: tier.rpd().clone(),
        };

        let result = GetTierInfoResult {
            provider: provider.to_string(),
            tier: tier_info,
        };

        debug!(
            tier_name = result.tier.name,
            rpm = ?result.tier.rpm,
            "Tier information retrieved"
        );

        Ok(Json(result))
    }
}
