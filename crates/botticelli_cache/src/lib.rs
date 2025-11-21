//! Command result caching with TTL support.
//!
//! This crate provides caching infrastructure for bot command results,
//! reducing API calls and improving response times.

#![warn(missing_docs)]

mod cache;

pub use cache::{CacheEntry, CommandCache, CommandCacheConfig, CommandCacheConfigBuilder};
