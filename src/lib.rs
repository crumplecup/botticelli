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

#[cfg(feature = "tui")]
mod tui;

// CLI module (private - exports re-exported at crate level)
mod cli;

// Re-export core types
pub use core::{
    GenerateRequest, GenerateResponse, Input, MediaSource, Message, Output, Role, ToolCall,
};

#[cfg(feature = "database")]
pub use database::{
    // Database row types
    ActExecutionRow,
    ActInputRow,
    ContentGenerationRepository,
    ContentGenerationRow,
    DatabaseError,
    DatabaseErrorKind,
    DatabaseResult,
    ModelResponse,
    NarrativeExecutionRow,
    NewActExecutionRow,
    NewActInputRow,
    NewContentGenerationRow,
    NewModelResponse,
    NewNarrativeExecutionRow,
    PostgresContentGenerationRepository,
    PostgresNarrativeRepository,
    SerializableModelResponse,
    UpdateContentGenerationRow,
    // Re-export schema tables for migration tools and tests
    act_inputs,
    // Schema documentation functions (Phase 5)
    assemble_prompt,
    is_content_focus,
    reflect_table_schema,
    // Schema inference functions (automatic table creation from JSON)
    infer_column_type,
    infer_schema,
    resolve_type_conflict,
    create_inferred_table,
    ColumnDefinition,
    InferredSchema,
    // Content generation functions
    create_content_table,
    // Content management functions
    delete_content,
    delete_response,
    // Discord schema tables
    discord_channels,
    discord_guild_members,
    discord_guilds,
    discord_member_roles,
    discord_roles,
    discord_users,
    // Connection and utility functions
    establish_connection,
    generate_schema_prompt,
    get_content_by_id,
    get_recent_responses,
    get_response_by_id,
    get_responses_by_model,
    list_content,
    media_references,
    narrative_executions,
    promote_content,
    run_migrations,
    store_error,
    store_response,
    update_content_metadata,
    update_review_status,
    DISCORD_PLATFORM_CONTEXT,
    JSON_FORMAT_REQUIREMENTS,
};

// Re-export PgConnection for processor use
#[cfg(feature = "database")]
pub use diesel::pg::PgConnection;

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
pub use models::{
    ClientContent, ClientContentMessage, FunctionCall, FunctionResponse, GenerationConfig,
    GeminiClient, GeminiError, GeminiErrorKind, GeminiLiveClient, GoAway, InlineData,
    InlineDataPart, LiveRateLimiter, LiveSession, LiveToolCall, LiveToolCallCancellation,
    MediaChunk, ModelTurn, Part, RealtimeInput, RealtimeInputMessage, ServerContent,
    ServerMessage, SetupComplete, SetupConfig, SetupMessage, SystemInstruction, TextPart,
    TieredGemini, Tool, ToolResponse, ToolResponseMessage, Turn, UsageMetadata,
};

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
    ActConfig,
    ActExecution,
    // Processor infrastructure
    ActProcessor,
    ExecutionFilter,
    ExecutionStatus,
    ExecutionSummary,
    InMemoryNarrativeRepository,
    Narrative,
    NarrativeError,
    NarrativeErrorKind,
    NarrativeExecution,
    NarrativeExecutor,
    NarrativeMetadata,
    NarrativeProvider,
    NarrativeRepository,
    NarrativeToc,
    ProcessorContext,
    ProcessorRegistry,
    VideoMetadata,
    // Extraction utilities
    extract_json,
    extract_toml,
    parse_json,
    parse_toml,
};

#[cfg(feature = "database")]
pub use narrative::ContentGenerationProcessor;

// Re-export rate limiting types
pub use rate_limit::{
    BoticelliConfig, HeaderRateLimitDetector, ProviderConfig, RateLimiter, RateLimiterGuard,
    RetryableError, Tier, TierConfig,
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
    // Client and handler
    BoticelliBot,
    BoticelliHandler,
    // Diesel models
    ChannelRow,
    ChannelType,
    // JSON models (for narrative processors)
    DiscordChannelJson,
    // Processors
    DiscordChannelProcessor,
    // Error handling
    DiscordError,
    DiscordErrorKind,
    DiscordErrorResult,
    DiscordGuildJson,
    DiscordGuildMemberJson,
    DiscordGuildMemberProcessor,
    DiscordGuildProcessor,
    DiscordMemberRoleJson,
    DiscordMemberRoleProcessor,
    // Repository
    DiscordRepository,
    DiscordResult,
    DiscordRoleJson,
    DiscordRoleProcessor,
    DiscordUserJson,
    DiscordUserProcessor,
    GuildMemberRow,
    GuildRow,
    NewChannel,
    NewGuild,
    NewGuildMember,
    NewMemberRole,
    NewRole,
    NewUser,
    RoleRow,
    UserRow,
    // Conversion utilities
    parse_channel_type,
    parse_iso_timestamp,
};

// Re-export CLI types
pub use cli::{Cli, Commands, RateLimitOptions};

#[cfg(feature = "discord")]
pub use cli::DiscordCommands;

#[cfg(feature = "database")]
pub use cli::ContentCommands;

// Re-export TUI types
#[cfg(feature = "tui")]
pub use tui::{run_tui, App, AppMode, TuiError, TuiErrorKind};
