//! Library function tools - expose botticelli workspace functions as MCP tools.
//!
//! This module demonstrates the "universal tooling" strategy: wrapping library
//! functions as MCP tools so LLMs can call them directly and compose workflows.

use crate::rmcp_server::helpers::to_mcp_error;
use crate::rmcp_server::BotticelliServer;
use botticelli_narrative::validator::Validator;
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

/// Parameters for selecting a model.
#[cfg(feature = "llm")]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct SelectModelParams {
    /// Selection strategy
    strategy: SelectionStrategy,
}

/// Result from model selection.
#[cfg(feature = "llm")]
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct SelectModelResult {
    /// Selected model ID
    model_id: String,
    
    /// Strategy used for selection
    strategy: String,
}

/// Parameters for inferring schema from JSON.
#[cfg(feature = "database")]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct InferSchemaParams {
    /// JSON sample data
    json: String,
}

/// Inferred column information.
#[cfg(feature = "database")]
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct InferredColumn {
    /// Column name
    name: String,
    
    /// SQL type
    sql_type: String,
    
    /// Whether the column is nullable
    nullable: bool,
}

/// Result from schema inference.
#[cfg(feature = "database")]
#[derive(Debug, Clone, Serialize, JsonSchema, derive_getters::Getters, elicitation::Elicit)]
pub struct InferSchemaResult {
    /// Inferred columns (sorted by name)
    columns: Vec<InferredColumn>,
}

#[tool_router(router = library_tool_router, vis = "pub")]
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

    /// Select optimal model based on criteria.
    ///
    /// Exposes `botticelli_models::ModelSelector` as an MCP tool.
    ///
    /// Available with the `llm` feature.
    #[cfg(feature = "llm")]
    #[instrument(skip(self, params), fields(strategy = ?params.strategy()))]
    pub async fn select_model(
        &self,
        Parameters(params): Parameters<SelectModelParams>,
    ) -> Result<Json<SelectModelResult>, rmcp::ErrorData> {
        use botticelli_models::{ModelSelector};
        
        debug!(strategy = ?params.strategy(), "Selecting model");

        let selector = ModelSelector::new();
        let model_id = selector
            .select(params.strategy())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Borrowed("No model matches the selection criteria"),
                    None,
                )
            })?;

        let result = SelectModelResult {
            model_id: model_id.to_string(),
            strategy: format!("{:?}", params.strategy()),
        };

        debug!(model_id = %result.model_id, "Model selected");

        Ok(Json(result))
    }

    /// Infer database schema from JSON sample data.
    ///
    /// Exposes `botticelli_database::infer_schema` as an MCP tool.
    ///
    /// Available with the `database` feature.
    #[cfg(feature = "database")]
    #[instrument(skip(self, params), fields(json_len = params.json().len()))]
    pub async fn infer_schema(
        &self,
        Parameters(params): Parameters<InferSchemaParams>,
    ) -> Result<Json<InferSchemaResult>, rmcp::ErrorData> {
        use botticelli_database::infer_schema;
        
        let json = params.json();
        debug!(json_len = json.len(), "Inferring schema from JSON");

        // Parse JSON
        let value: serde_json::Value = serde_json::from_str(json).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Invalid JSON: {}", e)),
                None,
            )
        })?;

        // Infer schema
        let schema = infer_schema(&value).map_err(|e| to_mcp_error(e, "Schema inference failed"))?;

        // Convert to result type
        let mut columns: Vec<InferredColumn> = schema
            .fields
            .iter()
            .map(|(name, col_def)| InferredColumn {
                name: name.clone(),
                sql_type: col_def.pg_type.clone(),
                nullable: col_def.nullable,
            })
            .collect();

        // Sort columns by name for consistent output
        columns.sort_by(|a, b| a.name.cmp(&b.name));

        let result = InferSchemaResult {
            columns,
        };

        debug!(
            column_count = result.columns.len(),
            "Schema inferred"
        );

        Ok(Json(result))
    }
}
