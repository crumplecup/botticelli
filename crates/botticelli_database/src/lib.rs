//! Storage backends for Botticelli.
//!
//! Provides [`RedbStorage`] (default, embedded) and [`PostgresStorage`]
//! (optional, via the `postgres` feature). Both implement [`BotStorage`].
//!
//! Also provides [`BotStorageTableQueryRegistry`] which adapts any
//! [`BotStorage`] to the [`TableQueryRegistry`] trait used by the narrative
//! executor.

#![forbid(unsafe_code)]

mod table_query;
pub use table_query::BotStorageTableQueryRegistry;

#[cfg(feature = "redb")]
mod redb_storage;
#[cfg(feature = "redb")]
pub use redb_storage::RedbStorage;

#[cfg(feature = "postgres")]
mod postgres_storage;
#[cfg(feature = "postgres")]
pub use postgres_storage::{PostgresConnectError, PostgresStorage};

#[cfg(any(feature = "redb", feature = "postgres"))]
mod env;
#[cfg(any(feature = "redb", feature = "postgres"))]
pub use env::open_storage_from_env;
