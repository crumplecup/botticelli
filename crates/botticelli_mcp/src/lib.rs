//! Model Context Protocol (MCP) server for Botticelli.
//!
//! This crate provides an MCP server that exposes Botticelli's capabilities
//! as standardized tools and resources that LLMs can use.
//!
//! # Features
//!
//! - **Tools**: Functions LLMs can call (DB queries, narrative execution, etc.)
//! - **Resources**: Data sources LLMs can read (content, narratives, etc.)
//! - **Prompts**: Reusable prompt templates
//!
//! # Usage
//!
//! ```no_run
//! use botticelli_mcp::BotticelliServer;
//! use rmcp::{ServerHandler, ServiceExt};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let server = BotticelliServer::builder().build()?;
//!
//!     // Use rmcp's serve method with stdio transport
//!     let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
//!     server.serve((stdin, stdout)).await?;
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod conversation;
mod create_narrative;
mod dialog_resource;
mod discord_tools;
mod echo;
mod elicit_bool;
mod elicit_number;
mod elicit_select;
mod elicit_text;
mod elicitation_protocol;
mod execution;
mod export_metrics;
mod modify_narrative;
mod partial;
mod query_content;
mod resources;
mod rmcp_server;
mod save_narrative;
mod scene;
mod server_info;
mod session_tools;
pub mod tools;
mod validate_narrative;

pub use conversation::{Attachment, ConversationSession, ConversationTurn, SessionState};
pub use create_narrative::{CreateNarrativeParams, CreateNarrativeResult};
pub use dialog_resource::DialogResource;
pub use discord_tools::{
    DiscordAuthor, DiscordChannelInfo, DiscordGetChannelsParams, DiscordGetChannelsResult,
    DiscordGetGuildInfoParams, DiscordGetGuildInfoResult, DiscordGetMessagesParams,
    DiscordGetMessagesResult, DiscordMessageInfo, DiscordPostMessageParams,
    DiscordPostMessageResult,
};
pub use echo::{EchoParams, EchoResult};
pub use elicit_bool::{ElicitBoolParams, ElicitBoolResult};
pub use elicit_number::{ElicitNumberParams, ElicitNumberResult};
pub use elicit_select::{ElicitSelectParams, ElicitSelectResult};
pub use elicit_text::{ElicitTextParams, ElicitTextResult};
pub use elicitation_protocol::{AgentProtocol, ElicitationProvider, HumanProtocol};
pub use execution::{
    ExecuteActParams, ExecuteActResult, ExecuteNarrativeParams, ExecuteNarrativeResult,
    GenerateParams, GenerateResult,
};
pub use export_metrics::{ExportMetricsParams, ExportMetricsResult, MetricsFormat};
pub use modify_narrative::{ModifyNarrativeParams, ModifyNarrativeResult};
pub use partial::{PartialAct, PartialNarrative, PartialNarrativeBuilder};
pub use query_content::{QueryContentParams, QueryContentResult};
#[cfg(feature = "database")]
pub use resources::ContentResource;
pub use resources::{NarrativeResource, ResourceInfo, ResourceRegistry};
pub use rmcp_server::{BotticelliServer, BotticelliServerBuilder};
pub use save_narrative::{SaveNarrativeParams, SaveNarrativeResult};
pub use scene::{
    CreateSceneParams, CreateSceneResult, DeleteSceneParams, DeleteSceneResult, ListScenesParams,
    ListScenesResult, UpdateSceneParams, UpdateSceneResult,
};
pub use server_info::ServerInfoResult;
pub use session_tools::{
    ApplyValidationFixesParams, ApplyValidationFixesResult, CarouselLevel, CarouselSummary,
    CreateNarrativeSessionParams, CreateNarrativeSessionResult, ElicitActParams,
    ElicitActParamsBuilder, ElicitActResult, ElicitCarouselParams, ElicitCarouselResult,
    ElicitMetadataParams, ElicitMetadataParamsBuilder, ElicitMetadataResult,
    FinalizeNarrativeParams, FinalizeNarrativeResult, GetNarrativeStateParams,
    GetNarrativeStateResult, NarrativeAnalysis, NarrativeStateSummary, StateFormat,
    ValidateNarrativeSessionParams, ValidateNarrativeSessionResult, ValidationIssue,
    ValidationSeverity,
};
pub use tools::{
    Act, ActMetrics, ElicitActInput, ElicitMetadataInput, ElicitationHelper, ExecutionMetrics,
    FinalizeNarrativeInput, LlmSamplerOperations, MetricsSummary, NarrativeHelper,
    NarrativeRegistry, PrometheusMetrics, SamplingCoordinator, SamplingHelper, SamplingResult,
    ToolRegistry,
};
pub use validate_narrative::{
    ValidateNarrativeParams, ValidateNarrativeParamsBuilder, ValidateNarrativeResult,
    ValidationError, ValidationLocation, ValidationWarning,
};

// MCP tool DTOs - Parameters and results for AI API (always available)
pub use rmcp_server::{
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
    // Budget<TierConfig> primitives
    BudgetNewTierConfigParams, BudgetResetWindowsTierConfigParams, BudgetResetWindowsTierConfigResult,
    BudgetCanAffordTierConfigParams, BudgetCanAffordTierConfigResult,
    BudgetConsumeTierConfigParams, BudgetConsumeTierConfigResult,
    BudgetRemainingTierConfigParams, BudgetRemainingTierConfigResult,
    // Budget<OpenAITier> primitives
    BudgetNewOpenAITierParams, BudgetResetWindowsOpenAITierParams, BudgetResetWindowsOpenAITierResult,
    BudgetCanAffordOpenAITierParams, BudgetCanAffordOpenAITierResult,
    BudgetConsumeOpenAITierParams, BudgetConsumeOpenAITierResult,
    BudgetRemainingOpenAITierParams, BudgetRemainingOpenAITierResult,
    // Validation primitives
    ValidateDiscordParams, ValidateTomlParams, ValidateTomlResult,
};

// Gemini feature
#[cfg(feature = "gemini")]
pub use rmcp_server::{
    // Budget<GeminiTier> primitives
    BudgetNewGeminiTierParams, BudgetResetWindowsGeminiTierParams, BudgetResetWindowsGeminiTierResult,
    BudgetCanAffordGeminiTierParams, BudgetCanAffordGeminiTierResult,
    BudgetConsumeGeminiTierParams, BudgetConsumeGeminiTierResult,
    BudgetRemainingGeminiTierParams, BudgetRemainingGeminiTierResult,
};

// Anthropic feature
#[cfg(feature = "anthropic")]
pub use rmcp_server::{
    // Budget<AnthropicTier> primitives
    BudgetNewAnthropicTierParams, BudgetResetWindowsAnthropicTierParams, BudgetResetWindowsAnthropicTierResult,
    BudgetCanAffordAnthropicTierParams, BudgetCanAffordAnthropicTierResult,
    BudgetConsumeAnthropicTierParams, BudgetConsumeAnthropicTierResult,
    BudgetRemainingAnthropicTierParams, BudgetRemainingAnthropicTierResult,
};

// Discord feature
#[cfg(feature = "discord")]
pub use rmcp_server::{
    BotRegistryHasPlatformParams, BotRegistryHasPlatformResult, BotRegistryNewParams,
    BotRegistryPlatformsParams, BotRegistryPlatformsResult,
    ConvertArgsToStringsParams, ConvertArgsToStringsResult, ConvertSecurityErrorParams,
    HashmapToParamsParams, HashmapToParamsResult,
};

// LLM feature (incomplete tools commented out)
// #[cfg(feature = "llm")]
// pub use rmcp_server::{SelectModelParams, SelectModelResult};

// Database feature
#[cfg(feature = "database")]
pub use rmcp_server::{
    AssembleNarrativeActPromptsParams, InferSchemaParams, InferSchemaResult, InferredColumn,
    MultiNarrativeFromFileWithDbParams, NarrativeFromFileWithDbParams,
};

// Model provider features
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "groq",
    feature = "huggingface",
    feature = "ollama"
))]
pub use rmcp_server::CountTokensParams;

#[cfg(feature = "gemini")]
pub use rmcp_server::GeminiGenerateParams;
#[cfg(feature = "anthropic")]
pub use rmcp_server::AnthropicGenerateParams;
#[cfg(feature = "groq")]
pub use rmcp_server::GroqGenerateParams;
#[cfg(feature = "huggingface")]
pub use rmcp_server::HuggingFaceGenerateParams;
#[cfg(feature = "ollama")]
pub use rmcp_server::OllamaGenerateParams;

// Tool helper functions (from tools/)
pub use tools::narrative_validation_helpers::{
    add_helpful_comments, auto_fix_common_issues, format_toml, format_validation_result,
};
