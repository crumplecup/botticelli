//! Tests for command result caching.

use botticelli_cache::{CommandCache, CommandCacheConfig, CommandCacheConfigBuilder};
use serde_json::json;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_cache_insert_and_get() {
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), json!("123456"));

    let value = json!({"members": 100, "channels": 10});

    cache.insert("discord", "server.get_stats", &args, value.clone(), Some(60));

    let entry = cache.get("discord", "server.get_stats", &args).unwrap();
    assert_eq!(entry.value(), &value);
}

#[test]
fn test_cache_miss() {
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    let args = HashMap::new();
    let result = cache.get("discord", "server.get_stats", &args);

    assert!(result.is_none());
}

#[test]
fn test_cache_expiration() {
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), json!("123456"));

    let value = json!({"members": 100});

    // Insert with 1 second TTL
    cache.insert("discord", "server.get_stats", &args, value, Some(1));

    // Should be available immediately
    assert!(cache.get("discord", "server.get_stats", &args).is_some());

    // Wait for expiration
    sleep(Duration::from_secs(2));

    // Should be expired
    assert!(cache.get("discord", "server.get_stats", &args).is_none());
}

#[test]
fn test_cache_different_args() {
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    let mut args1 = HashMap::new();
    args1.insert("guild_id".to_string(), json!("123456"));

    let mut args2 = HashMap::new();
    args2.insert("guild_id".to_string(), json!("789012"));

    cache.insert("discord", "server.get_stats", &args1, json!({"members": 100}), None);
    cache.insert("discord", "server.get_stats", &args2, json!({"members": 200}), None);

    let value1 = cache.get("discord", "server.get_stats", &args1).unwrap().value().clone();
    let value2 = cache.get("discord", "server.get_stats", &args2).unwrap().value().clone();

    assert_eq!(value1["members"], 100);
    assert_eq!(value2["members"], 200);
}

#[test]
fn test_cache_cleanup_expired() {
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), json!("123456"));

    // Insert with 1 second TTL
    cache.insert("discord", "server.get_stats", &args, json!({"members": 100}), Some(1));

    assert_eq!(cache.len(), 1);

    // Wait for expiration
    sleep(Duration::from_secs(2));

    let removed = cache.cleanup_expired();
    assert_eq!(removed, 1);
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_cache_lru_eviction() {
    let config = CommandCacheConfigBuilder::default()
        .default_ttl(300)
        .max_size(2)
        .enabled(true)
        .build()
        .unwrap();
    let mut cache = CommandCache::new(config);

    let mut args1 = HashMap::new();
    args1.insert("id".to_string(), json!("1"));

    let mut args2 = HashMap::new();
    args2.insert("id".to_string(), json!("2"));

    let mut args3 = HashMap::new();
    args3.insert("id".to_string(), json!("3"));

    cache.insert("discord", "cmd", &args1, json!({"data": 1}), None);
    cache.insert("discord", "cmd", &args2, json!({"data": 2}), None);

    assert_eq!(cache.len(), 2);

    // This should evict the LRU entry (args1)
    cache.insert("discord", "cmd", &args3, json!({"data": 3}), None);

    assert_eq!(cache.len(), 2);
    assert!(cache.get("discord", "cmd", &args1).is_none()); // Evicted
    assert!(cache.get("discord", "cmd", &args2).is_some()); // Still there
    assert!(cache.get("discord", "cmd", &args3).is_some()); // Just added
}

#[test]
fn test_cache_disabled() {
    let config = CommandCacheConfigBuilder::default()
        .default_ttl(300)
        .max_size(1000)
        .enabled(false)
        .build()
        .unwrap();
    let mut cache = CommandCache::new(config);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), json!("123456"));

    cache.insert("discord", "server.get_stats", &args, json!({"members": 100}), None);

    // Cache disabled, should return None
    assert!(cache.get("discord", "server.get_stats", &args).is_none());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_cache_clear() {
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), json!("123456"));

    cache.insert("discord", "server.get_stats", &args, json!({"members": 100}), None);

    assert_eq!(cache.len(), 1);

    cache.clear();

    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}
