//! Discord integration for Botticelli.
//!
//! This module provides a complete Discord bot implementation using the Serenity library.
//! It enables Botticelli to:
//! - Connect to Discord servers (guilds)
//! - Listen for events (messages, commands, interactions)
//! - Post narrative content to channels
//! - Store Discord data in the database for analytics and state management
//! - Respond to slash commands and interactions
//!
//! # Architecture
//!
//! The Discord integration follows Botticelli's layered architecture:
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
//! use botticelli::social::discord::BotticelliBot;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let bot = BotticelliBot::new(
//!         std::env::var("DISCORD_TOKEN")?,
//!         std::env::var("DATABASE_URL")?,
//!     ).await?;
//!
//!     bot.start().await?;
//!     Ok(())
//! }
//! ```

mod client;
mod conversions;
mod error;
mod handler;
mod json_models;
mod models;
mod repository;

// Public re-exports
pub use client::BotticelliBot;
pub use conversions::{NewMemberRole, parse_channel_type, parse_iso_timestamp};
pub use error::{DiscordError, DiscordErrorKind, DiscordResult as DiscordErrorResult};
pub use handler::BotticelliHandler;
pub use json_models::{
    DiscordChannelJson, DiscordGuildJson, DiscordGuildMemberJson, DiscordMemberRoleJson,
    DiscordRoleJson, DiscordUserJson,
};
pub use models::{
    ChannelRow, ChannelType, GuildMemberRow, GuildRow, NewChannel, NewGuild, NewGuildMember,
    NewRole, NewUser, RoleRow, UserRow,
};
// pub use processors::{
//     DiscordChannelProcessor, DiscordGuildMemberProcessor, DiscordGuildProcessor,
//     DiscordMemberRoleProcessor, DiscordRoleProcessor, DiscordUserProcessor,
// };
pub use repository::{DiscordRepository, DiscordResult};

// TODO: Uncomment exports as modules are implemented
// commands, poster
