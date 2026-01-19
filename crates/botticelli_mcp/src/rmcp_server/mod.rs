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
    ExtractJsonParams, ExtractTomlParams, GetTierInfoParams, GetTierInfoResult,
    MediaStorageDeleteParams, MediaStorageDeleteResult, MediaStorageExistsParams,
    MediaStorageExistsResult, MediaStorageGetUrlParams, MediaStorageGetUrlResult,
    MediaStorageRetrieveParams, MediaStorageRetrieveResult, MediaStorageStoreParams,
    MediaStorageStoreResult, MultiNarrativeFromFileParams, NarrativeFromFileParams,
    NarrativeFromTomlStrParams, StateManagerNewParams, TierDailyQuotaParams, TierDailyQuotaResult,
    TierInfo, TierInputCostParams, TierInputCostResult, TierMaxConcurrentParams,
    TierMaxConcurrentResult, TierNameParams, TierNameResult, TierOutputCostParams,
    TierOutputCostResult, TierRpdParams, TierRpdResult, TierRpmParams, TierRpmResult,
    TierTpdParams, TierTpdResult, TierTpmParams, TierTpmResult, ValidateDiscordParams,
    ValidateTomlParams, ValidateTomlResult,
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
