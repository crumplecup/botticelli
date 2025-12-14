#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Chat interface for user/botticelli interaction.
//!
//! Provides trait-based abstraction for building chat interfaces across
//! different platforms (TUI, web, mobile). Enables users to direct
//! botticelli on tasks like narrative generation, bot assignment, and
//! social media scheduling.

mod chat_config;
mod command;
mod executor;
mod input;
mod interface;
mod message;
mod model_selection;
mod parser;
mod response;
mod services;
mod startup;
mod state;

mod config;
mod elicitation;
mod sampling;
mod sampling_integration;

pub use botticelli_mcp::{
    ElicitationDialog, NarrativeElicitor, PartialAct, PartialNarrative, PartialNarrativeBuilder,
};
pub use chat_config::ChatConfig;
pub use command::{BotCommand, Command, NarrativeCommand, SocialCommand};
pub use config::{
    ChatAppConfig, ConfigBuilder, EnvironmentConfig, EnvironmentMode, McpClientConfig,
    McpServerConfig, ObservabilityConfig, PostgresConfig,
};
pub use elicitation::{
    ActElicitor, CarouselElicitor, ElicitationSession, InputElicitor, MetadataElicitor,
    ValidationElicitor,
};
pub use sampling::ChatLlmSampler;
pub use sampling_integration::SamplingIntegration;
pub use executor::{CommandExecutor, NarrativeState};
pub use input::UserInput;
pub use interface::ChatInterface;
pub use message::Message;
pub use model_selection::ChatSession;
pub use parser::parse_intent;
pub use response::Response;
pub use services::ServiceContainer;

#[cfg(feature = "cli")]
pub use startup::startup_sequence;
pub use state::ConversationState;
