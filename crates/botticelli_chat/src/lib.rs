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
#[cfg(feature = "cli")]
mod conversation_loop;
mod elicitation;
#[cfg(feature = "cli")]
mod mcp_chat_host;
mod tool_handler;

#[cfg(feature = "cli")]
mod sampling;
#[cfg(feature = "cli")]
mod sampling_integration;

pub use botticelli_mcp::{
    ConversationSession, ElicitationDialog, PartialAct, PartialNarrative, PartialNarrativeBuilder,
};
pub use chat_config::ChatConfig;
pub use command::{BotCommand, Command, NarrativeCommand, SocialCommand};
pub use config::{
    ChatAppConfig, ConfigBuilder, EnvironmentConfig, EnvironmentMode, McpClientConfig,
    McpServerConfig, ObservabilityConfig, PostgresConfig,
};
pub use elicitation::{
    ActApproach, ActDefinition, BotCommandConfig, CarouselConfig, DocumentInputConfig,
    HistoryRetentionMode, InputType, MediaInputConfig, MediaSource, NarrativeMetadata,
    NarrativeReferenceConfig, OutputFormat, TableQueryConfig, TextInputConfig,
    create_mcp_client_for_dialog, elicit_acts, elicit_carousel, elicit_inputs, elicit_metadata,
    elicit_validation,
};

pub use executor::{CommandExecutor, NarrativeState};
pub use input::UserInput;
pub use interface::ChatInterface;
pub use message::Message;
pub use model_selection::ChatSession;
pub use parser::parse_intent;
pub use response::Response;

#[cfg(feature = "cli")]
pub use sampling::ChatLlmSampler;
#[cfg(feature = "cli")]
pub use sampling_integration::SamplingIntegration;

pub use services::ServiceContainer;

#[cfg(feature = "cli")]
pub use mcp_chat_host::McpChatHost;

#[cfg(feature = "cli")]
pub use conversation_loop::ConversationLoop;
#[cfg(feature = "cli")]
pub use startup::startup_sequence;
pub use state::ConversationState;
pub use tool_handler::ToolCallHandler;
