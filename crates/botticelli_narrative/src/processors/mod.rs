//! Built-in processor implementations.
//!
//! This module provides concrete processors for common use cases:
//! - Discord data extraction and storage
//! - JSON extraction
//! - Database insertion

#[cfg(feature = "database")]
mod discord_processors;

#[cfg(feature = "database")]
pub use discord_processors::{DiscordChannelProcessor, DiscordGuildProcessor};
