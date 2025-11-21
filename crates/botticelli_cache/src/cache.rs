//! Command result cache implementation.

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Cache entry with value and expiration.
#[derive(Debug, Clone, Getters)]
pub struct CacheEntry {
    value: JsonValue,
    created_at: Instant,
    ttl: Duration,
}

impl CacheEntry {
    /// Check if this entry is expired.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    /// Get remaining time until expiration.
    pub fn time_remaining(&self) -> Option<Duration> {
        self.ttl.checked_sub(self.created_at.elapsed())
    }
}

/// Cache key for command results.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    platform: String,
    command: String,
    args_hash: u64,
}

impl CacheKey {
    fn new(platform: &str, command: &str, args: &HashMap<String, JsonValue>) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        
        // Create stable hash of args
        let mut sorted_keys: Vec<_> = args.keys().collect();
        sorted_keys.sort();
        
        for key in sorted_keys {
            key.hash(&mut hasher);
            // Hash JSON value as string for stability
            if let Ok(s) = serde_json::to_string(&args[key]) {
                s.hash(&mut hasher);
            }
        }
        
        Self {
            platform: platform.to_string(),
            command: command.to_string(),
            args_hash: hasher.finish(),
        }
    }
}

/// Configuration for command cache.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_setters::Setters, derive_builder::Builder)]
#[setters(prefix = "with_")]
pub struct CommandCacheConfig {
    /// Default TTL for cached entries (seconds)
    #[serde(default = "default_ttl")]
    default_ttl: u64,
    
    /// Maximum cache size (number of entries)
    #[serde(default = "default_max_size")]
    max_size: usize,
    
    /// Whether caching is enabled
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_ttl() -> u64 {
    300 // 5 minutes
}

fn default_max_size() -> usize {
    1000
}

fn default_enabled() -> bool {
    true
}

impl Default for CommandCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: default_ttl(),
            max_size: default_max_size(),
            enabled: default_enabled(),
        }
    }
}

/// Cache for bot command results.
///
/// Stores command results with TTL-based expiration. Cache keys are
/// derived from platform, command name, and a hash of arguments.
///
/// # Example
///
/// ```
/// use botticelli_cache::{CommandCache, CommandCacheConfig};
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let config = CommandCacheConfig::default();
/// let mut cache = CommandCache::new(config);
///
/// let mut args = HashMap::new();
/// args.insert("guild_id".to_string(), json!("123456"));
///
/// // Cache a result
/// cache.insert("discord", "server.get_stats", &args, json!({"members": 100}), Some(60));
///
/// // Retrieve cached result
/// if let Some(entry) = cache.get("discord", "server.get_stats", &args) {
///     println!("Cached: {:?}", entry.value());
/// }
/// ```
pub struct CommandCache {
    config: CommandCacheConfig,
    entries: HashMap<CacheKey, CacheEntry>,
    access_order: Vec<CacheKey>,
}

impl CommandCache {
    /// Create a new command cache with configuration.
    pub fn new(config: CommandCacheConfig) -> Self {
        tracing::debug!(
            default_ttl = config.default_ttl,
            max_size = config.max_size,
            enabled = config.enabled,
            "Creating new CommandCache"
        );
        Self {
            config,
            entries: HashMap::new(),
            access_order: Vec::new(),
        }
    }

    /// Insert a command result into the cache.
    ///
    /// # Arguments
    ///
    /// * `platform` - Platform name (e.g., "discord")
    /// * `command` - Command name (e.g., "server.get_stats")
    /// * `args` - Command arguments
    /// * `value` - Result value to cache
    /// * `ttl_seconds` - TTL in seconds (uses default if None)
    #[tracing::instrument(
        skip(self, args, value),
        fields(
            platform,
            command,
            ttl_seconds,
            cache_size = self.entries.len()
        )
    )]
    pub fn insert(
        &mut self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
        value: JsonValue,
        ttl_seconds: Option<u64>,
    ) {
        if !self.config.enabled {
            tracing::debug!("Cache disabled, skipping insert");
            return;
        }

        let key = CacheKey::new(platform, command, args);
        let ttl = Duration::from_secs(ttl_seconds.unwrap_or(self.config.default_ttl));

        let entry = CacheEntry {
            value,
            created_at: Instant::now(),
            ttl,
        };

        // Evict if at capacity
        if self.entries.len() >= self.config.max_size && !self.entries.contains_key(&key) {
            self.evict_lru();
        }

        // Track access order for LRU
        if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
            self.access_order.remove(pos);
        }
        self.access_order.push(key.clone());

        tracing::debug!(
            cache_hit = self.entries.contains_key(&key),
            ttl = ?ttl,
            "Inserted entry into cache"
        );

        self.entries.insert(key, entry);
    }

    /// Get a cached command result.
    ///
    /// Returns None if:
    /// - Entry doesn't exist
    /// - Entry is expired
    /// - Cache is disabled
    #[tracing::instrument(
        skip(self, args),
        fields(
            platform,
            command,
            cache_size = self.entries.len()
        )
    )]
    pub fn get(
        &mut self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Option<&CacheEntry> {
        if !self.config.enabled {
            tracing::debug!("Cache disabled, returning None");
            return None;
        }

        let key = CacheKey::new(platform, command, args);

        // Check if entry exists and is not expired
        let entry = self.entries.get(&key)?;
        if entry.is_expired() {
            tracing::debug!("Cache entry expired, removing");
            self.entries.remove(&key);
            if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
                self.access_order.remove(pos);
            }
            return None;
        }

        // Update access order for LRU
        if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
            let key_clone = self.access_order.remove(pos);
            self.access_order.push(key_clone);
        }

        tracing::debug!(
            time_remaining = ?entry.time_remaining(),
            "Cache hit"
        );

        self.entries.get(&key)
    }

    /// Remove expired entries from cache.
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.entries.len();

        self.entries.retain(|key, entry| {
            let keep = !entry.is_expired();
            if !keep
                && let Some(pos) = self.access_order.iter().position(|k| k == key)
            {
                self.access_order.remove(pos);
            }
            keep
        });

        let removed = before - self.entries.len();
        if removed > 0 {
            tracing::info!(removed, remaining = self.entries.len(), "Cleaned up expired cache entries");
        }
        removed
    }

    /// Clear all cache entries.
    pub fn clear(&mut self) {
        let count = self.entries.len();
        self.entries.clear();
        self.access_order.clear();
        tracing::info!(cleared = count, "Cleared cache");
    }

    /// Get number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Evict least recently used entry.
    fn evict_lru(&mut self) {
        if let Some(key) = self.access_order.first().cloned() {
            tracing::debug!(
                platform = %key.platform,
                command = %key.command,
                "Evicting LRU entry"
            );
            self.entries.remove(&key);
            self.access_order.remove(0);
        }
    }
}

impl Default for CommandCache {
    fn default() -> Self {
        Self::new(CommandCacheConfig::default())
    }
}
