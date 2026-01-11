//! Social media platform integrations for Botticelli.
//!
//! This module provides integrations with various social media platforms,
//! enabling Botticelli to post narrative content, respond to events, and
//! interact with users across different platforms.
//!
//! # Bot Command Execution
//!
//! The crate provides a platform-agnostic command execution system:
//! - `BotCommandExecutor` - Trait for implementing platform-specific commands
//! - `BotCommandRegistry` - Registry for managing multiple platform executors
//! - `BotCommandError` - Error types for command failures
//!
//! # Platform Support
//!
//! Each platform is feature-gated and lives in its own submodule:
//! - `discord` - Discord bot integration (requires `discord` feature)
//! - `telegram` - Telegram bot integration (requires `telegram` feature, not yet implemented)
//! - `reddit` - Reddit integration (requires `reddit` feature, not yet implemented)
//!
//! Platform implementations follow a common pattern:
//! - Platform-specific error types
//! - Diesel models for database persistence
//! - Repository layer for data access
//! - Client/handler for platform API interaction
//! - Bot command executor for narrative integration

#![warn(missing_docs)]

#[cfg(feature = "database")]
mod bot_commands;
#[cfg(feature = "database")]
mod database;
#[cfg(feature = "database")]
mod secure_bot_executor;
#[cfg(feature = "database")]
mod secure_executor;

#[cfg(feature = "discord")]
mod discord;

// Export bot command infrastructure (requires database feature)
#[cfg(feature = "database")]
pub use bot_commands::BotCommandRegistryImpl;
#[cfg(feature = "database")]
pub use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
#[cfg(feature = "database")]
pub use database::DatabaseCommandExecutor;

// Export secure executor (requires database feature)
#[cfg(feature = "database")]
pub use secure_bot_executor::SecureBotExecutor;
#[cfg(feature = "database")]
pub use secure_executor::{ExecutionResult, SecureBotCommandExecutor};

// Export Discord-specific types (feature-gated)
#[cfg(feature = "discord")]
pub use discord::{
    BotticelliBot, BotticelliHandler, ChannelRow, ChannelType, DiscordChannelJson,
    DiscordCommandExecutor, DiscordError, DiscordErrorKind, DiscordErrorResult, DiscordGuildJson,
    DiscordGuildMemberJson, DiscordMemberRoleJson, DiscordRepository, DiscordResult,
    DiscordRoleJson, DiscordUserJson, GuildMemberRow, GuildRow, NewChannel, NewGuild,
    NewGuildBuilder, NewGuildMember, NewMemberRole, NewRole, NewUser, RoleRow, UserRow,
    parse_channel_type, parse_iso_timestamp,
};
