//! Discord integration for Boticelli.
//!
//! This module provides a complete Discord bot implementation using the Serenity library.
//! It enables Boticelli to:
//! - Connect to Discord servers (guilds)
//! - Listen for events (messages, commands, interactions)
//! - Post narrative content to channels
//! - Store Discord data in the database for analytics and state management
//! - Respond to slash commands and interactions
//!
//! # Architecture
//!
//! The Discord integration follows Boticelli's layered architecture:
//!
//! ## Data Layer
//! - **models**: Diesel models for Discord entities (guilds, channels, users, messages, etc.)
//! - **repository**: Database operations following the repository pattern
//!
//! ## Integration Layer
//! - **client**: Serenity client setup and lifecycle management
//! - **handler**: Event handler implementing Serenity's EventHandler trait
//! - **error**: Discord-specific error types
//!
//! ## Feature Layer
//! - **commands**: Slash command implementations
//! - **poster**: Narrative-to-Discord posting functionality
//!
//! # Usage
//!
//! Available with the `discord` feature.
//!
//! ```rust,ignore
//! use boticelli::social::discord::BoticelliBot;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let bot = BoticelliBot::new(
//!         std::env::var("DISCORD_TOKEN")?,
//!         std::env::var("DATABASE_URL")?,
//!     ).await?;
//!
//!     bot.start().await?;
//!     Ok(())
//! }
//! ```

// Module declarations (private)
mod client;
mod error;
mod handler;
mod json_models;
pub mod models;
mod repository;
// mod commands;
// mod poster;

// Public re-exports
pub use client::BoticelliBot;
pub use error::{DiscordError, DiscordErrorKind, DiscordResult as DiscordErrorResult};
pub use handler::BoticelliHandler;
pub use json_models::{
    DiscordChannelJson, DiscordGuildJson, DiscordGuildMemberJson, DiscordMemberRoleJson,
    DiscordRoleJson, DiscordUserJson,
};
pub use models::{
    ChannelRow, ChannelType, GuildMemberRow, GuildRow, NewChannel, NewGuild, NewGuildMember,
    NewRole, NewUser, RoleRow, UserRow,
};
pub use repository::{DiscordRepository, DiscordResult};

// TODO: Uncomment exports as modules are implemented
// commands, poster
