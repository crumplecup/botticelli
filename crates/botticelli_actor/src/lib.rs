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
//! use botticelli_actor::{Actor, ActorConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ActorConfig::from_file("actor.toml")?;
//! let actor = Actor::builder()
//!     .config(config)
//!     .build()?;
//!
//! actor.execute().await?;
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod config;
mod content;
mod error;
mod knowledge;
mod platform;
mod skill;

pub use config::{
    ActorCacheConfig, ActorCacheConfigBuilder, ActorConfig, ActorConfigBuilder, ActorSettings,
    ActorSettingsBuilder, CacheStrategy, ExecutionConfig, ExecutionConfigBuilder, SkillConfig,
    SkillConfigBuilder,
};
pub use content::{Content, ContentBuilder, MediaAttachment, MediaAttachmentBuilder, MediaType};
pub use error::{ActorError, ActorErrorKind, ActorResult};
pub use knowledge::KnowledgeTable;
pub use platform::{
    PlatformMetadata, PlatformMetadataBuilder, PlatformResult, PostId, ScheduleId,
    SocialMediaPlatform,
};
pub use skill::{Skill, SkillContext, SkillInfo, SkillOutput, SkillRegistry, SkillResult};
