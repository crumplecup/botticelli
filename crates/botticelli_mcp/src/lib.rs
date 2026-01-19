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
pub use rmcp_server::{SelectModelParams, SelectModelResult};

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
