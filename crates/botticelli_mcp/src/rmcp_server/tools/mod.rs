//! MCP tool implementations organized by category.

mod cache;
mod core;
mod core_primitives;
#[cfg(feature = "discord")]
mod discord;
mod elicitation;
mod execution;
mod extraction_tools;
mod library;
#[cfg(any(feature = "gemini", feature = "anthropic", feature = "groq", feature = "huggingface", feature = "ollama"))]
mod models;
mod narrative;
mod rate_limit;
mod scene;
mod security;
#[cfg(feature = "discord")]
mod social;
mod storage;

use crate::rmcp_server::BotticelliServer;
use rmcp::tool_router;

// Re-export tool DTOs (parameters and results)
// Always available
pub use cache::{
    CacheCleanupParams, CacheCleanupResult, CacheClearParams, CacheEntryIsExpiredParams,
    CacheEntryIsExpiredResult, CacheEntryTimeRemainingParams, CacheEntryTimeRemainingResult,
    CacheEvictLruParams, CacheIsEmptyParams, CacheIsEmptyResult, CacheKeyNewParams, CacheLenParams,
    CacheLenResult, CommandCacheNewParams,
};
pub use core_primitives::{
    CoreBudgetApplyResult, CoreBudgetApplyRpdParams, CoreBudgetApplyRpmParams,
    CoreBudgetApplyTpmParams, CoreBudgetMergeParams, CoreBudgetMergeResult,
    CoreBudgetValidateParams, CoreGetTokenizerParams, CoreGetTokenizerResult,
    CoreInitObservabilityWithConfigParams, CoreInputHistoryRetentionParams,
    CoreInputHistoryRetentionResult, CoreInputWithHistoryRetentionParams,
    CoreInputWithHistoryRetentionResult, CoreTokenUsageCalculateCostParams,
    CoreTokenUsageCalculateCostResult, CoreTokenUsageNewParams,
};
pub use extraction_tools::{ExtractJsonParams, ExtractTomlParams};
pub use library::{GetTierInfoParams, GetTierInfoResult, TierInfo, ValidateTomlParams, ValidateTomlResult};
pub use narrative::{
    MultiNarrativeFromFileParams, NarrativeFromFileParams, NarrativeFromTomlStrParams,
    StateManagerNewParams,
};
pub use rate_limit::{
    TierDailyQuotaParams, TierDailyQuotaResult, TierInputCostParams, TierInputCostResult,
    TierMaxConcurrentParams, TierMaxConcurrentResult, TierOutputCostParams, TierOutputCostResult,
    TierRpdParams, TierRpdResult, TierRpmParams, TierRpmResult, TierTpdParams, TierTpdResult,
    TierTpmParams, TierTpmResult, TierNameParams, TierNameResult,
    RateLimitForModelParams, RateLimitFromTierParams, RateLimitUnlimitedParams,
    RateLimitFromFileParams, RateLimitGetTierParams, RateLimitGetTierResult,
    // Budget<TierConfig> wrappers
    BudgetNewTierConfigParams, BudgetResetWindowsTierConfigParams, BudgetResetWindowsTierConfigResult,
    BudgetCanAffordTierConfigParams, BudgetCanAffordTierConfigResult,
    BudgetConsumeTierConfigParams, BudgetConsumeTierConfigResult,
    BudgetRemainingTierConfigParams, BudgetRemainingTierConfigResult,
    // Budget<OpenAITier> wrappers
    BudgetNewOpenAITierParams, BudgetResetWindowsOpenAITierParams, BudgetResetWindowsOpenAITierResult,
    BudgetCanAffordOpenAITierParams, BudgetCanAffordOpenAITierResult,
    BudgetConsumeOpenAITierParams, BudgetConsumeOpenAITierResult,
    BudgetRemainingOpenAITierParams, BudgetRemainingOpenAITierResult,
};
#[cfg(feature = "gemini")]
pub use rate_limit::{
    // Budget<GeminiTier> wrappers
    BudgetNewGeminiTierParams, BudgetResetWindowsGeminiTierParams, BudgetResetWindowsGeminiTierResult,
    BudgetCanAffordGeminiTierParams, BudgetCanAffordGeminiTierResult,
    BudgetConsumeGeminiTierParams, BudgetConsumeGeminiTierResult,
    BudgetRemainingGeminiTierParams, BudgetRemainingGeminiTierResult,
};
#[cfg(feature = "anthropic")]
pub use rate_limit::{
    // Budget<AnthropicTier> wrappers
    BudgetNewAnthropicTierParams, BudgetResetWindowsAnthropicTierParams, BudgetResetWindowsAnthropicTierResult,
    BudgetCanAffordAnthropicTierParams, BudgetCanAffordAnthropicTierResult,
    BudgetConsumeAnthropicTierParams, BudgetConsumeAnthropicTierResult,
    BudgetRemainingAnthropicTierParams, BudgetRemainingAnthropicTierResult,
};
pub use security::ValidateDiscordParams;
#[cfg(feature = "discord")]
pub use social::{
    BotRegistryHasPlatformParams, BotRegistryHasPlatformResult, BotRegistryNewParams,
    BotRegistryPlatformsParams, BotRegistryPlatformsResult, BotRegistryWithCacheParams,
    ConvertArgsToStringsParams, ConvertArgsToStringsResult, ConvertSecurityErrorParams,
    HashmapToParamsParams, HashmapToParamsResult,
};
pub use storage::{
    MediaStorageDeleteParams, MediaStorageDeleteResult, MediaStorageExistsParams,
    MediaStorageExistsResult, MediaStorageGetUrlParams, MediaStorageGetUrlResult,
    MediaStorageRetrieveParams, MediaStorageRetrieveResult, MediaStorageStoreParams,
    MediaStorageStoreResult, MediaTypeAsStrParams, MediaTypeAsStrResult,
    StorageComputeHashParams, StorageComputeHashResult, StorageGetPathParams,
    StorageGetPathResult, StorageNewParams, StorageVerifyHashParams,
};

// LLM feature
#[cfg(feature = "llm")]
pub use library::{SelectModelParams, SelectModelResult};

// Database feature
#[cfg(feature = "database")]
pub use library::{InferSchemaParams, InferSchemaResult, InferredColumn};
#[cfg(feature = "database")]
pub use narrative::{
    AssembleNarrativeActPromptsParams, MultiNarrativeFromFileWithDbParams,
    NarrativeFromFileWithDbParams,
};

// Model provider features
#[cfg(any(feature = "gemini", feature = "anthropic", feature = "groq", feature = "huggingface", feature = "ollama"))]
pub use models::{
    CountTokensParams,
};
#[cfg(feature = "gemini")]
pub use models::GeminiGenerateParams;
#[cfg(feature = "anthropic")]
pub use models::AnthropicGenerateParams;
#[cfg(feature = "groq")]
pub use models::GroqGenerateParams;
#[cfg(feature = "huggingface")]
pub use models::HuggingFaceGenerateParams;
#[cfg(feature = "ollama")]
pub use models::OllamaGenerateParams;

/// Public wrapper to combine all tool routers from separate modules.
impl BotticelliServer {
    /// Get a combined tool router instance for this server type.
    ///
    /// This combines tool routers from all modules into a single router.
    pub(crate) fn create_tool_router() -> rmcp::handler::server::tool::ToolRouter<Self> {
        // Start with core (already has correct signatures)
        let router = Self::core_tool_router() + Self::cache_tool_router();
        
        // TODO: Add storage and other modules as they're refactored
        // + Self::storage_tool_router()
        
        // TODO: Add other modules as they're refactored to use Parameters<> and Result<Json<>>
        // + cache::cache_tool_router()
        // + core_primitives::core_primitives_tool_router()
        // + elicitation::elicitation_tool_router()
        // + extraction_tools::extraction_tool_router()
        // + library::library_tool_router()
        // + narrative::narrative_tool_router()
        // + rate_limit::rate_limit_tool_router()
        // + scene::scene_tool_router()
        // + security::security_tool_router()
        // + storage::storage_tool_router()
        
        // #[cfg(feature = "discord")]
        // let router = router + discord::discord_tool_router() + social::social_tool_router();

        router
    }
}
