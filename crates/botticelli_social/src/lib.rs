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

mod bot_commands;
mod secure_executor;

#[cfg(feature = "discord")]
mod discord;

// Export bot command infrastructure (always available)
pub use bot_commands::{
    BotCommandError, BotCommandErrorKind, BotCommandExecutor, BotCommandRegistryImpl,
    BotCommandResult,
};

// Export secure executor (always available)
pub use secure_executor::{ExecutionResult, SecureBotCommandExecutor};

// Export Discord-specific types (feature-gated)
#[cfg(feature = "discord")]
pub use discord::{
    BotticelliBot, BotticelliHandler, ChannelRow, ChannelType, DiscordChannelJson,
    DiscordCommandExecutor, DiscordError, DiscordErrorKind, DiscordErrorResult, DiscordGuildJson,
    DiscordGuildMemberJson, DiscordMemberRoleJson, DiscordRepository, DiscordResult,
    DiscordRoleJson, DiscordUserJson, GuildMemberRow, GuildRow, NewChannel, NewGuild,
    NewGuildMember, NewMemberRole, NewRole, NewUser, RoleRow, UserRow, parse_channel_type,
    parse_iso_timestamp,
};
