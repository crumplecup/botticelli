//! Trait definitions for the Botticelli LLM API library.
//!
//! This crate provides the core traits and capability traits that define
//! the Botticelli interface.

mod bot_server;
mod capabilities;
mod chat_host;
mod chat_service;
mod discord_events;
mod driver;
mod elicitation_dialog;
mod executor;
mod health;
mod llm_sampler;
mod media_storage;
mod metadata;
mod narrative_elicitor;
mod provider;
mod registry;
mod registry_traits;
mod repository;
mod retry;
mod tier;

pub use bot_server::{BotActor, BotServer};
pub use capabilities::{
    Audio, BatchGeneration, DocumentProcessing, Embeddings, JsonMode, Streaming, TokenCounting,
    ToolCalling, Video, Vision,
};
pub use chat_host::ChatHost;
pub use chat_service::ChatService;
pub use discord_events::{DiscordEventProcessor, EventResult};
pub use driver::BotticelliDriver;
pub use elicitation_dialog::ElicitationDialog;
pub use executor::BotCommandExecutor;
pub use health::Health;
pub use llm_sampler::LlmSamplerOperations;
pub use media_storage::MediaStorage;
pub use metadata::Metadata;
pub use narrative_elicitor::NarrativeElicitor;
pub use provider::LlmProvider;
pub use registry::RegistryOperations;
pub use registry_traits::{
    DatabaseRegistryOperations, ElicitationRegistryOperations, NarrativeRegistryOperations,
    NarrativeStorageOperations,
};
pub use repository::{
    ActProcessor, BotCommandRegistry, ContentGenerationRepository, ContentRepository,
    NarrativeProvider, NarrativeRepository, ProcessorTrait, TableQueryRegistry, TableView,
};
pub use retry::RetryableError;
pub use tier::Tier;
