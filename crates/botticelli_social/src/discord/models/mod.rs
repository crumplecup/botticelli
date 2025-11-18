//! Diesel models for Discord database tables.
//!
//! This module provides three types of structs for each Discord entity:
//! - `*Row` - Database row representation (Queryable, for SELECT queries)
//! - `New*` - Insert representation (Insertable, for INSERT queries)
//! - `*` - Business logic representation (domain models, future use)
//!
//! Following Botticelli patterns, these models map directly to the database schema
//! and provide type-safe access to Discord data.

mod channel;
mod guild;
mod member;
mod role;
mod user;

// Re-export all types
pub use channel::{ChannelRow, ChannelType, NewChannel};
pub use guild::{GuildRow, NewGuild};
pub use member::{GuildMemberRow, NewGuildMember};
pub use role::{NewRole, RoleRow};
pub use user::{NewUser, UserRow};
