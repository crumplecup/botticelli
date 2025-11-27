//! Platform implementations for social media services.

pub mod noop;

#[cfg(feature = "discord")]
pub mod discord;

pub use noop::NoOpPlatform;

#[cfg(feature = "discord")]
pub use discord::{DiscordPlatform, DiscordPlatformBuilder};
