//! Core primitive delegation wrappers.
//!
//! Orchestrator wrappers for botticelli_core primitives including
//! observability, budget config, token counting, and builders.

use crate::rmcp_server::BotticelliServer;
use botticelli_core::{
    BudgetConfig, HistoryRetention, Input, ObservabilityConfig, TokenUsageData,
};
use botticelli_error::{ConfigError, ObservabilityResult, TokenCountingResult};
use elicitation::Elicit;
use rmcp::tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

// ============================================================================
// DTOs - All parameter and result structs
// ============================================================================

/// Parameters for initializing observability with custom config.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreInitObservabilityWithConfigParams {
    /// Observability configuration
    pub config: ObservabilityConfig,
}

/// Parameters for validating budget config.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreBudgetValidateParams {
    /// Budget configuration to validate
    pub config: BudgetConfig,
}

/// Parameters for applying budget to RPM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreBudgetApplyRpmParams {
    /// Budget configuration
    pub config: BudgetConfig,
    /// Requests per minute limit
    pub rpm: u64,
}

/// Parameters for applying budget to TPM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreBudgetApplyTpmParams {
    /// Budget configuration
    pub config: BudgetConfig,
    /// Tokens per minute limit
    pub tpm: u64,
}

/// Parameters for applying budget to RPD.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreBudgetApplyRpdParams {
    /// Budget configuration
    pub config: BudgetConfig,
    /// Requests per day limit
    pub rpd: u64,
}

/// Result of applying budget.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreBudgetApplyResult {
    /// Adjusted value after applying budget
    pub adjusted_value: u64,
}

/// Parameters for merging budget configs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreBudgetMergeParams {
    /// First budget configuration
    pub config1: BudgetConfig,
    /// Second budget configuration  
    pub config2: BudgetConfig,
}

/// Result of merging budgets.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreBudgetMergeResult {
    /// Merged budget configuration
    pub merged_config: BudgetConfig,
}

/// Parameters for getting tokenizer.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreGetTokenizerParams {
    /// Model name (e.g., "gpt-4")
    pub model: String,
}

/// Result of getting tokenizer.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CoreGetTokenizerResult {
    /// Success indicator (Arc<CoreBPE> not serializable)
    pub success: bool,
    /// Model name
    pub model: String,
}

/// Parameters for creating token usage data.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreTokenUsageNewParams {
    /// Input tokens
    pub input_tokens: u64,
    /// Output tokens
    pub output_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
}

/// Parameters for calculating token cost.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreTokenUsageCalculateCostParams {
    /// Token usage data
    pub usage: TokenUsageData,
    /// Input cost per token
    pub input_cost_per_token: f64,
    /// Output cost per token
    pub output_cost_per_token: f64,
}

/// Result of cost calculation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreTokenUsageCalculateCostResult {
    /// Total cost
    pub total_cost: f64,
}

/// Parameters for getting input history retention.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreInputHistoryRetentionParams {
    /// Input to query
    pub input: Input,
}

/// Result of getting history retention.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreInputHistoryRetentionResult {
    /// History retention setting
    pub retention: HistoryRetention,
}

/// Parameters for setting input history retention.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreInputWithHistoryRetentionParams {
    /// Input to modify
    pub input: Input,
    /// New retention setting
    pub retention: HistoryRetention,
}

/// Result of setting history retention.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CoreInputWithHistoryRetentionResult {
    /// Modified input
    pub input: Input,
}

// ============================================================================
// Orchestrator Wrappers
// ============================================================================

impl BotticelliServer {
    /// Initialize OpenTelemetry observability with default config.
    #[tool]
    #[instrument(skip(self), fields(tool = "core_init_observability"))]
    pub fn core_init_observability(&self) -> ObservabilityResult<()> {
        tracing::debug!("Delegating to botticelli_core::init_observability");
        
        let result = botticelli_core::init_observability();
        
        match &result {
            Ok(_) => tracing::debug!("Observability initialized"),
            Err(e) => tracing::error!(error = ?e, "Observability init failed"),
        }
        
        result
    }

    /// Initialize OpenTelemetry observability with custom config.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_init_observability_with_config"))]
    pub fn core_init_observability_with_config(
        &self,
        params: CoreInitObservabilityWithConfigParams,
    ) -> ObservabilityResult<()> {
        tracing::debug!("Delegating to botticelli_core::init_observability_with_config");
        
        let result = botticelli_core::init_observability_with_config(params.config);
        
        match &result {
            Ok(_) => tracing::debug!("Observability initialized with config"),
            Err(e) => tracing::error!(error = ?e, "Observability init failed"),
        }
        
        result
    }

    /// Shutdown observability and flush all telemetry.
    #[tool]
    #[instrument(skip(self), fields(tool = "core_shutdown_observability"))]
    pub fn core_shutdown_observability(&self) {
        tracing::debug!("Delegating to botticelli_core::shutdown_observability");
        botticelli_core::shutdown_observability();
        tracing::debug!("Observability shutdown complete");
    }

    /// Get default budget multiplier value.
    #[tool]
    #[instrument(skip(self), fields(tool = "core_default_multiplier"))]
    pub fn core_default_multiplier(&self) -> f64 {
        tracing::debug!("Delegating to botticelli_core::default_multiplier_tool");
        let result = botticelli_core::default_multiplier_tool();
        tracing::debug!(multiplier = result, "Default multiplier retrieved");
        result
    }

    /// Create a budget config builder.
    #[tool]
    #[instrument(skip(self), fields(tool = "core_budget_builder"))]
    pub fn core_budget_builder(&self) -> botticelli_core::BudgetConfigBuilder {
        tracing::debug!("Delegating to BudgetConfig::builder");
        let builder = BudgetConfig::builder();
        tracing::debug!("Budget builder created");
        builder
    }

    /// Validate budget configuration multipliers.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_budget_validate"))]
    pub fn core_budget_validate(&self, params: CoreBudgetValidateParams) -> Result<(), ConfigError> {
        tracing::debug!("Delegating to BudgetConfig::validate");
        
        let result = params.config.validate();
        
        match &result {
            Ok(_) => tracing::debug!("Budget config valid"),
            Err(e) => tracing::error!(error = ?e, "Budget validation failed"),
        }
        
        result
    }

    /// Apply budget to requests per minute limit.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_budget_apply_rpm", rpm = params.rpm))]
    pub fn core_budget_apply_rpm(&self, params: CoreBudgetApplyRpmParams) -> CoreBudgetApplyResult {
        tracing::debug!("Delegating to BudgetConfig::apply_rpm");
        
        let adjusted = params.config.apply_rpm(params.rpm);
        
        tracing::debug!(original = params.rpm, adjusted, "RPM budget applied");
        CoreBudgetApplyResult { adjusted_value: adjusted }
    }

    /// Apply budget to tokens per minute limit.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_budget_apply_tpm", tpm = params.tpm))]
    pub fn core_budget_apply_tpm(&self, params: CoreBudgetApplyTpmParams) -> CoreBudgetApplyResult {
        tracing::debug!("Delegating to BudgetConfig::apply_tpm");
        
        let adjusted = params.config.apply_tpm(params.tpm);
        
        tracing::debug!(original = params.tpm, adjusted, "TPM budget applied");
        CoreBudgetApplyResult { adjusted_value: adjusted }
    }

    /// Apply budget to requests per day limit.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_budget_apply_rpd", rpd = params.rpd))]
    pub fn core_budget_apply_rpd(&self, params: CoreBudgetApplyRpdParams) -> CoreBudgetApplyResult {
        tracing::debug!("Delegating to BudgetConfig::apply_rpd");
        
        let adjusted = params.config.apply_rpd(params.rpd);
        
        tracing::debug!(original = params.rpd, adjusted, "RPD budget applied");
        CoreBudgetApplyResult { adjusted_value: adjusted }
    }

    /// Merge two budget configs, taking minimum of each multiplier.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_budget_merge"))]
    pub fn core_budget_merge(&self, params: CoreBudgetMergeParams) -> CoreBudgetMergeResult {
        tracing::debug!("Delegating to BudgetConfig::merge");
        
        let merged = params.config1.merge(&params.config2);
        
        tracing::debug!("Budget configs merged");
        CoreBudgetMergeResult { merged_config: merged }
    }

    /// Get tokenizer for model.
    ///
    /// Note: Returns success indicator since Arc<CoreBPE> is not serializable.
    /// Use this to verify tokenizer availability.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_get_tokenizer", model = %params.model))]
    pub fn core_get_tokenizer(&self, params: CoreGetTokenizerParams) -> TokenCountingResult<CoreGetTokenizerResult> {
        tracing::debug!("Delegating to botticelli_core::get_tokenizer");
        
        match botticelli_core::get_tokenizer(&params.model) {
            Ok(_) => {
                tracing::debug!("Tokenizer retrieved");
                Ok(CoreGetTokenizerResult {
                    success: true,
                    model: params.model,
                })
            }
            Err(e) => {
                tracing::error!(error = ?e, "Tokenizer retrieval failed");
                Err(e)
            }
        }
    }

    /// Create new token usage data.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_token_usage_new"))]
    pub fn core_token_usage_new(&self, params: CoreTokenUsageNewParams) -> TokenUsageData {
        tracing::debug!("Delegating to TokenUsageData::new");
        
        let usage = TokenUsageData::new(
            params.input_tokens,
            params.output_tokens,
            params.total_tokens,
        );
        
        tracing::debug!("Token usage data created");
        usage
    }

    /// Create token usage builder.
    #[tool]
    #[instrument(skip(self), fields(tool = "core_token_usage_builder"))]
    pub fn core_token_usage_builder(&self) -> botticelli_core::TokenUsageDataBuilder {
        tracing::debug!("Delegating to TokenUsageData::builder");
        let builder = TokenUsageData::builder();
        tracing::debug!("Token usage builder created");
        builder
    }

    /// Calculate token cost.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_token_usage_calculate_cost"))]
    pub fn core_token_usage_calculate_cost(
        &self,
        params: CoreTokenUsageCalculateCostParams,
    ) -> CoreTokenUsageCalculateCostResult {
        tracing::debug!("Delegating to TokenUsageData::calculate_cost");
        
        let cost = params.usage.calculate_cost(
            params.input_cost_per_token,
            params.output_cost_per_token,
        );
        
        tracing::debug!(cost, "Token cost calculated");
        CoreTokenUsageCalculateCostResult { total_cost: cost }
    }

    /// Create message builder.
    #[tool]
    #[instrument(skip(self), fields(tool = "core_message_builder"))]
    pub fn core_message_builder(&self) -> botticelli_core::MessageBuilder {
        tracing::debug!("Delegating to Message::builder");
        let builder = botticelli_core::Message::builder();
        tracing::debug!("Message builder created");
        builder
    }

    /// Create stream chunk builder.
    #[tool]
    #[instrument(skip(self), fields(tool = "core_stream_chunk_builder"))]
    pub fn core_stream_chunk_builder(&self) -> botticelli_core::StreamChunkBuilder {
        tracing::debug!("Delegating to StreamChunk::builder");
        let builder = botticelli_core::StreamChunk::builder();
        tracing::debug!("Stream chunk builder created");
        builder
    }

    /// Create generate request builder.
    #[tool]
    #[instrument(skip(self), fields(tool = "core_generate_request_builder"))]
    pub fn core_generate_request_builder(&self) -> botticelli_core::GenerateRequestBuilder {
        tracing::debug!("Delegating to GenerateRequest::builder");
        let builder = botticelli_core::GenerateRequest::builder();
        tracing::debug!("Generate request builder created");
        builder
    }

    /// Create generate response builder.
    #[tool]
    #[instrument(skip(self), fields(tool = "core_generate_response_builder"))]
    pub fn core_generate_response_builder(&self) -> botticelli_core::GenerateResponseBuilder {
        tracing::debug!("Delegating to GenerateResponse::builder");
        let builder = botticelli_core::GenerateResponse::builder();
        tracing::debug!("Generate response builder created");
        builder
    }

    /// Get history retention setting from input.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_input_history_retention"))]
    pub fn core_input_history_retention(
        &self,
        params: CoreInputHistoryRetentionParams,
    ) -> CoreInputHistoryRetentionResult {
        tracing::debug!("Delegating to Input::history_retention");
        
        let retention = params.input.history_retention();
        
        tracing::debug!(?retention, "History retention retrieved");
        CoreInputHistoryRetentionResult { retention }
    }

    /// Set history retention on input.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "core_input_with_history_retention"))]
    pub fn core_input_with_history_retention(
        &self,
        params: CoreInputWithHistoryRetentionParams,
    ) -> CoreInputWithHistoryRetentionResult {
        tracing::debug!("Delegating to Input::with_history_retention");
        
        let input = params.input.with_history_retention(params.retention);
        
        tracing::debug!("History retention set");
        CoreInputWithHistoryRetentionResult { input }
    }
}
