//! MCP tool implementations organized by category.

mod cache;
mod core;
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

// Empty impl block required for #[tool_router] macro
impl BotticelliServer {}

// This macro gathers all tool methods from the separate module files
#[tool_router]
impl BotticelliServer {}
