//! boticelli: unified interface for bot-driver interaction with multiple LLM APIs.

#![forbid(unsafe_code)]
//!
//! # Design Philosophy
//!
//! This library uses a trait-based architecture where capabilities are exposed through
//! separate traits. This allows:
//! - Models to implement only what they support
//! - Compile-time checking of capabilities
//! - Clear API boundaries for different features
//!
//! # Core Traits
//!
//! - [`BoticelliDriver`] - Core trait all backends must implement (basic generation)
//!
//! # Capability Traits
//!
//! Optional traits that backends may implement:
//! - [`Streaming`] - Streaming response support
//! - [`Embeddings`] - Text embedding generation
//! - [`Vision`] - Image input support (multimodal vision)
//! - [`Audio`] - Audio input/output support (speech, audio generation)
//! - [`Video`] - Video input/output support
//! - [`DocumentProcessing`] - Document understanding (PDF, DOCX, etc.)
//! - [`ToolUse`] - Function/tool calling
//! - [`JsonMode`] - Structured JSON output
//! - [`TokenCounting`] - Token counting utilities
//! - [`BatchGeneration`] - Batch request processing
//! - [`Metadata`] - Model metadata and limits
//! - [`Health`] - Health check support
//!
//! # Example
//!
//! ```rust,ignore
//! use boticelli::{BoticelliDriver, Streaming, GenerateRequest};
//!
//! async fn process<T>(client: &T, req: &GenerateRequest)
//! where
//!     T: BoticelliDriver + Streaming,
//! {
//!     // Can use both core and streaming capabilities
//!     let stream = client.generate_stream(req).await.unwrap();
//!     // ...
//! }
//! ```

mod core;
mod error;
mod interface;
mod models;

#[cfg(feature = "database")]
mod database;

mod narrative;
mod rate_limit;
mod storage;

#[cfg(feature = "discord")]
mod social;

// Re-export core types
pub use core::{
    GenerateRequest, GenerateResponse, Input, MediaSource, Message, Output, Role, ToolCall,
};

#[cfg(feature = "database")]
pub use database::{
    // Connection and utility functions
    establish_connection, store_response, store_error, get_response_by_id,
    get_responses_by_model, get_recent_responses, delete_response, run_migrations,
    // Database row types
    ActExecutionRow, ActInputRow, DatabaseError, DatabaseErrorKind, DatabaseResult,
    ModelResponse, NarrativeExecutionRow, NewActExecutionRow, NewActInputRow,
    NewModelResponse, NewNarrativeExecutionRow, PostgresNarrativeRepository,
    SerializableModelResponse,
    // Re-export schema tables for migration tools and tests
    act_inputs, media_references, narrative_executions,
    // Discord schema tables
    discord_channels, discord_guild_members, discord_guilds,
    discord_member_roles, discord_roles, discord_users,
};

// Re-export error types
pub use error::{
    BackendError, BoticelliError, BoticelliErrorKind, BoticelliResult, ConfigError, HttpError,
    JsonError, NotImplementedError,
};

// Re-export storage types
pub use storage::{
    FileSystemStorage, MediaMetadata, MediaReference, MediaStorage, MediaType, StorageError,
    StorageErrorKind,
};

// Re-export model-specific error types
// Re-export model implementations
#[cfg(feature = "gemini")]
pub use models::{GeminiClient, GeminiError, GeminiErrorKind, TieredGemini};

// Re-export core trait
pub use interface::BoticelliDriver;

// Re-export capability traits
pub use interface::{
    Audio, BatchGeneration, DocumentProcessing, Embeddings, Health, JsonMode, Metadata, Streaming,
    TokenCounting, ToolUse, Video, Vision,
};

// Re-export capability-specific types
pub use interface::{
    FinishReason, HealthStatus, ModelMetadata, StreamChunk, ToolDefinition, ToolResult,
};

// Re-export narrative types
pub use narrative::{
    ActConfig, ActExecution, ExecutionFilter, ExecutionStatus, ExecutionSummary,
    InMemoryNarrativeRepository, Narrative, NarrativeError, NarrativeErrorKind, NarrativeExecution,
    NarrativeExecutor, NarrativeMetadata, NarrativeProvider, NarrativeRepository, NarrativeToc,
    VideoMetadata,
    // Extraction utilities
    extract_json, extract_toml, parse_json, parse_toml,
    // Processor infrastructure
    ActProcessor, ProcessorRegistry,
};

// Re-export rate limiting types
pub use rate_limit::{
    BoticelliConfig, HeaderRateLimitDetector, ProviderConfig, RateLimiter, RateLimiterGuard, Tier,
    TierConfig,
};

// Re-export provider-specific tier enums
#[cfg(feature = "anthropic")]
pub use rate_limit::AnthropicTier;
#[cfg(feature = "gemini")]
pub use rate_limit::GeminiTier;
pub use rate_limit::OpenAITier;

// Re-export social media platform types
#[cfg(feature = "discord")]
pub use social::discord::{
    // Diesel models
    ChannelRow, ChannelType, GuildMemberRow, GuildRow, NewChannel, NewGuild, NewGuildMember,
    NewMemberRole, NewRole, NewUser, RoleRow, UserRow,
    // JSON models (for narrative processors)
    DiscordChannelJson, DiscordGuildJson, DiscordGuildMemberJson, DiscordMemberRoleJson,
    DiscordRoleJson, DiscordUserJson,
    // Conversion utilities
    parse_channel_type, parse_iso_timestamp,
    // Processors
    DiscordChannelProcessor, DiscordGuildMemberProcessor, DiscordGuildProcessor,
    DiscordMemberRoleProcessor, DiscordRoleProcessor, DiscordUserProcessor,
    // Repository
    DiscordRepository, DiscordResult,
    // Error handling
    DiscordError, DiscordErrorKind, DiscordErrorResult,
    // Client and handler
    BoticelliBot, BoticelliHandler,
};
