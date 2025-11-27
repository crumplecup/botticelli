//! Platform-agnostic actor system for social media automation.
//!
//! This crate provides the core abstractions for building automated social media
//! actors that can work across multiple platforms (Discord, Twitter, Bluesky, etc.).
//!
//! # Architecture
//!
//! - **Actors**: Configured bots that orchestrate skills and knowledge
//! - **Platforms**: Trait-based abstraction for social media APIs
//! - **Skills**: Reusable capabilities (scheduling, filtering, etc.)
//! - **Knowledge**: Database tables produced by narratives
//!
//! # Example
//!
//! ```no_run
//! use botticelli_actor::{Actor, ActorConfig, SkillRegistry, DiscordPlatform};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ActorConfig::from_file("actor.toml")?;
//! let platform = DiscordPlatform::new("token", "channel_id")?;
//! let registry = SkillRegistry::new();
//!
//! let actor = Actor::builder()
//!     .config(config)
//!     .skills(registry)
//!     .platform(Arc::new(platform))
//!     .build()?;
//!
//! // Execute requires database connection
//! // let mut conn = establish_connection()?;
//! // actor.execute(&mut conn).await?;
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod actor;
mod config;
mod content;
#[cfg(feature = "discord")]
mod discord_server;
mod error;
mod execution_tracker;
mod knowledge;
mod platform_trait;
pub mod platforms;
mod server;
mod server_config;
mod skill;
pub mod skills;
mod state_persistence;

pub use actor::{Actor, ActorBuilder, ExecutionResult, ExecutionResultBuilder};
pub use config::{
    ActorCacheConfig, ActorCacheConfigBuilder, ActorConfig, ActorConfigBuilder, ActorSettings,
    ActorSettingsBuilder, CacheStrategy, ExecutionConfig, ExecutionConfigBuilder, SkillConfig,
    SkillConfigBuilder,
};
pub use content::{
    Content, ContentBuilder, ContentPost, ContentPostBuilder, MediaAttachment,
    MediaAttachmentBuilder, MediaType,
};
pub use error::{ActorError, ActorErrorKind, ActorResult};
pub use execution_tracker::ActorExecutionTracker;
pub use knowledge::KnowledgeTable;
pub use platform_trait::{Platform, PlatformCapability, PlatformMessage, PlatformMetadata};
pub use server::{
    BasicActorServer, GenericActorManager, GenericContentPoster, JsonStatePersistence,
    SimpleTaskScheduler,
};
pub use server_config::{ActorInstanceConfig, ActorServerConfig, ScheduleConfig, ServerSettings};
pub use skill::{
    Skill, SkillContext, SkillContextBuilder, SkillInfo, SkillInfoBuilder, SkillOutput,
    SkillOutputBuilder, SkillRegistry, SkillResult,
};
pub use skills::{
    ContentFormatterSkill, ContentSchedulingSkill, ContentSelectionSkill, DuplicateCheckSkill,
    RateLimitingSkill,
};
pub use state_persistence::{DatabaseExecutionResult, DatabaseStatePersistence};

#[cfg(feature = "discord")]
pub use discord_server::{
    DiscordActorId, DiscordActorManager, DiscordActorServer, DiscordContentPoster, DiscordContext,
    DiscordServerState, DiscordTaskScheduler,
};

pub use platforms::NoOpPlatform;

#[cfg(feature = "discord")]
pub use platforms::{DiscordPlatform, DiscordPlatformBuilder};
