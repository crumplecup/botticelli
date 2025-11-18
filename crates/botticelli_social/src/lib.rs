//! Social media platform integrations for Botticelli.
//!
//! This module provides integrations with various social media platforms,
//! enabling Botticelli to post narrative content, respond to events, and
//! interact with users across different platforms.
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
//! - Integration with Botticelli's narrative system

#[cfg(feature = "discord")]
pub mod discord;
