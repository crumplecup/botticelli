//! Discord bot command executor (modular).
//!
//! This module implements the BotCommandExecutor trait for Discord,
//! routing commands to domain-specific submodules.

mod channels;
mod events;
mod executor;
mod forum;
mod members;
mod messages;
mod misc;
mod moderation;
mod reactions;
mod roles;
mod server;
mod threads;

pub use executor::DiscordCommandExecutor;
