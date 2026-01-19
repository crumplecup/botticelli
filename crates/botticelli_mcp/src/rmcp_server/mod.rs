//! RMCP-based MCP server implementation.
//!
//! The BotticelliServer struct holds all tool implementations and state
//! needed for MCP operations.

mod handler;
mod helpers;
mod server;
mod tools;

pub use server::{BotticelliServer, BotticelliServerBuilder};

// Always available tool DTOs
pub use tools::{
    // Cache primitives
    CacheCleanupParams, CacheCleanupResult, CacheClearParams, CacheEntryIsExpiredParams,
    CacheEntryIsExpiredResult, CacheEntryTimeRemainingParams, CacheEntryTimeRemainingResult,
    CacheEvictLruParams, CacheIsEmptyParams, CacheIsEmptyResult, CacheKeyNewParams, CacheLenParams,
    CacheLenResult, CommandCacheNewParams,
    // Core infrastructure primitives
    CoreBudgetApplyResult, CoreBudgetApplyRpdParams, CoreBudgetApplyRpmParams,
    CoreBudgetApplyTpmParams, CoreBudgetMergeParams, CoreBudgetMergeResult,
    CoreBudgetValidateParams, CoreGetTokenizerParams, CoreGetTokenizerResult,
    CoreInitObservabilityWithConfigParams, CoreInputHistoryRetentionParams,
    CoreInputHistoryRetentionResult, CoreInputWithHistoryRetentionParams,
    CoreInputWithHistoryRetentionResult, CoreTokenUsageCalculateCostParams,
    CoreTokenUsageCalculateCostResult, CoreTokenUsageNewParams,
    // Tool/library primitives
    ExtractJsonParams, ExtractTomlParams, GetTierInfoParams, GetTierInfoResult,
};

// Discord feature
#[cfg(feature = "discord")]
pub use tools::{
    // Social primitives
    BotRegistryHasPlatformParams, BotRegistryHasPlatformResult, BotRegistryNewParams,
    BotRegistryPlatformsParams, BotRegistryPlatformsResult, BotRegistryWithCacheParams,
    ConvertArgsToStringsParams, ConvertArgsToStringsResult, ConvertSecurityErrorParams,
    HashmapToParamsParams, HashmapToParamsResult,
};

// Always available tool DTOs (continued)
pub use tools::{
    // Storage primitives
    MediaStorageDeleteParams, MediaStorageDeleteResult, MediaStorageExistsParams,
    MediaStorageExistsResult, MediaStorageGetUrlParams, MediaStorageGetUrlResult,
    MediaStorageRetrieveParams, MediaStorageRetrieveResult, MediaStorageStoreParams,
    MediaStorageStoreResult, MediaTypeAsStrParams, MediaTypeAsStrResult, StorageComputeHashParams,
    StorageComputeHashResult, StorageGetPathParams, StorageGetPathResult, StorageNewParams,
    StorageVerifyHashParams,
    // Narrative primitives
    MultiNarrativeFromFileParams, NarrativeFromFileParams, NarrativeFromTomlStrParams,
    StateManagerNewParams,
    // Tier primitives
    TierDailyQuotaParams, TierDailyQuotaResult, TierInfo, TierInputCostParams, TierInputCostResult,
    TierMaxConcurrentParams, TierMaxConcurrentResult, TierNameParams, TierNameResult,
    TierOutputCostParams, TierOutputCostResult, TierRpdParams, TierRpdResult, TierRpmParams,
    TierRpmResult, TierTpdParams, TierTpdResult, TierTpmParams, TierTpmResult,
    // Rate limit config primitives
    RateLimitForModelParams, RateLimitFromTierParams, RateLimitUnlimitedParams,
    RateLimitFromFileParams, RateLimitGetTierParams, RateLimitGetTierResult,
    // Validation primitives
    ValidateDiscordParams, ValidateTomlParams, ValidateTomlResult,
};

// LLM feature
#[cfg(feature = "llm")]
pub use tools::{SelectModelParams, SelectModelResult};

// Database feature
#[cfg(feature = "database")]
pub use tools::{
    AssembleNarrativeActPromptsParams, InferSchemaParams, InferSchemaResult, InferredColumn,
    MultiNarrativeFromFileWithDbParams, NarrativeFromFileWithDbParams,
};

// Model provider features (any one enables CountTokensParams)
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "groq",
    feature = "huggingface",
    feature = "ollama"
))]
pub use tools::CountTokensParams;

#[cfg(feature = "gemini")]
pub use tools::GeminiGenerateParams;
#[cfg(feature = "anthropic")]
pub use tools::AnthropicGenerateParams;
#[cfg(feature = "groq")]
pub use tools::GroqGenerateParams;
#[cfg(feature = "huggingface")]
pub use tools::HuggingFaceGenerateParams;
#[cfg(feature = "ollama")]
pub use tools::OllamaGenerateParams;
