//! Trait definitions for the Botticelli LLM API library.
//!
//! This crate provides the core traits and capability traits that define
//! the Botticelli interface.

mod bot_server;
mod capabilities;
mod chat_host;
mod chat_service;
mod driver;
mod health;
mod metadata;
mod provider;
mod registry;
mod registry_traits;
mod repository;
mod retry;

pub use bot_server::{BotActor, BotServer};
pub use capabilities::{
    Audio, BatchGeneration, DocumentProcessing, Embeddings, JsonMode, Streaming, TokenCounting,
    ToolCalling, Video, Vision,
};
pub use chat_host::ChatHost;
pub use chat_service::ChatService;
pub use driver::BotticelliDriver;
pub use health::Health;
pub use metadata::Metadata;
pub use provider::LlmProvider;
pub use registry::RegistryOperations;
pub use registry_traits::{
    DatabaseRegistryOperations, ElicitationRegistryOperations, NarrativeRegistryOperations,
    NarrativeStorageOperations,
};
pub use repository::{ContentRepository, NarrativeRepository, TableQueryRegistry, TableView};
pub use retry::RetryableError;
