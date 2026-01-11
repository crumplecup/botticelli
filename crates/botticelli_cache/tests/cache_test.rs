//! Tests for the CommandCache implementation.

mod helpers;

use botticelli_cache::{CommandCache, CommandCacheConfig};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

#[test]
fn test_cache_insert_and_get() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing cache insert and get operations");
    
    let config = CommandCacheConfig::default()
        .with_default_ttl(10)
        .with_max_size(100);
    let mut cache = CommandCache::new(config);
    tracing::debug!(default_ttl = 10, max_size = 100, "Created cache");

    let mut args = HashMap::new();
    args.insert("param1".to_string(), json!("value1"));

    cache.insert("discord", "test.command", &args, json!("result"), Some(10));
    tracing::debug!("Inserted test entry");

    let entry = cache.get("discord", "test.command", &args);
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().value(), &json!("result"));
    tracing::debug!("Retrieved and verified entry");

    // Non-existent command should return None
    assert!(cache.get("discord", "other.command", &args).is_none());
    tracing::debug!("Verified non-existent entry returns None");
    
    tracing::info!("Cache insert and get test passed");
    Ok(())
}

#[test]
fn test_cache_expiration() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing cache expiration");
    
    let config = CommandCacheConfig::default().with_default_ttl(1); // 1 second TTL
    let mut cache = CommandCache::new(config);
    tracing::debug!(ttl_seconds = 1, "Created cache with short TTL");

    let mut args = HashMap::new();
    args.insert("param1".to_string(), json!("value1"));

    cache.insert("discord", "test.command", &args, json!("result"), Some(1));
    assert!(cache.get("discord", "test.command", &args).is_some());
    tracing::debug!("Entry inserted and verified present");

    // Wait for expiration
    std::thread::sleep(Duration::from_secs(2));
    tracing::debug!("Waited 2 seconds for expiration");

    // Should be expired now
    assert!(cache.get("discord", "test.command", &args).is_none());
    tracing::debug!("Verified entry expired");
    
    tracing::info!("Cache expiration test passed");
    Ok(())
}

#[test]
fn test_cache_clear() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing cache clear operation");
    
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    let mut args1 = HashMap::new();
    args1.insert("param1".to_string(), json!("value1"));
    let mut args2 = HashMap::new();
    args2.insert("param2".to_string(), json!("value2"));

    cache.insert("discord", "cmd1", &args1, json!("result1"), None);
    cache.insert("discord", "cmd2", &args2, json!("result2"), None);
    tracing::debug!(entry_count = 2, "Inserted test entries");

    assert_eq!(cache.len(), 2);

    cache.clear();
    tracing::debug!("Cleared cache");

    assert_eq!(cache.len(), 0);
    assert!(cache.get("discord", "cmd1", &args1).is_none());
    assert!(cache.get("discord", "cmd2", &args2).is_none());
    tracing::debug!("Verified cache is empty");
    
    tracing::info!("Cache clear test passed");
    Ok(())
}

#[test]
fn test_cache_len() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing cache length tracking");
    
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    assert_eq!(cache.len(), 0);
    tracing::debug!("Verified empty cache has length 0");

    let mut args1 = HashMap::new();
    args1.insert("param1".to_string(), json!("value1"));

    cache.insert("discord", "cmd1", &args1, json!("result1"), None);
    assert_eq!(cache.len(), 1);
    tracing::debug!(cache_len = 1, "Added first entry");

    let mut args2 = HashMap::new();
    args2.insert("param2".to_string(), json!("value2"));

    cache.insert("discord", "cmd2", &args2, json!("result2"), None);
    assert_eq!(cache.len(), 2);
    tracing::debug!(cache_len = 2, "Added second entry");
    
    tracing::info!("Cache length test passed");
    Ok(())
}

#[test]
fn test_cache_is_empty() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing cache is_empty check");
    
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    assert!(cache.is_empty());
    tracing::debug!("Verified new cache is empty");

    let mut args = HashMap::new();
    args.insert("param1".to_string(), json!("value1"));

    cache.insert("discord", "cmd1", &args, json!("result1"), None);
    assert!(!cache.is_empty());
    tracing::debug!("Verified cache not empty after insert");

    cache.clear();
    assert!(cache.is_empty());
    tracing::debug!("Verified cache empty after clear");
    
    tracing::info!("Cache is_empty test passed");
    Ok(())
}

#[test]
fn test_cache_update_existing_key() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing cache update of existing key");
    
    let config = CommandCacheConfig::default();
    let mut cache = CommandCache::new(config);

    let mut args = HashMap::new();
    args.insert("param1".to_string(), json!("value1"));

    cache.insert("discord", "cmd1", &args, json!("result1"), None);
    let entry = cache.get("discord", "cmd1", &args);
    assert_eq!(entry.unwrap().value(), &json!("result1"));
    tracing::debug!("Inserted and verified initial value");

    // Update with new value
    cache.insert("discord", "cmd1", &args, json!("result2"), None);
    let entry = cache.get("discord", "cmd1", &args);
    assert_eq!(entry.unwrap().value(), &json!("result2"));
    tracing::debug!("Updated and verified new value");
    
    tracing::info!("Cache update existing key test passed");
    Ok(())
}

#[test]
fn test_cache_cleanup_expired_entries() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing cleanup of expired entries");
    
    let config = CommandCacheConfig::default().with_default_ttl(1);
    let mut cache = CommandCache::new(config);
    tracing::debug!(ttl_seconds = 1, "Created cache with short TTL");

    let mut args1 = HashMap::new();
    args1.insert("param1".to_string(), json!("value1"));
    let mut args2 = HashMap::new();
    args2.insert("param2".to_string(), json!("value2"));

    cache.insert("discord", "cmd1", &args1, json!("result1"), Some(1));
    cache.insert("discord", "cmd2", &args2, json!("result2"), Some(1));

    assert_eq!(cache.len(), 2);
    tracing::debug!(entry_count = 2, "Inserted test entries");

    // Wait for expiration
    std::thread::sleep(Duration::from_secs(2));
    tracing::debug!("Waited 2 seconds for expiration");

    // Cleanup expired entries
    let removed = cache.cleanup_expired();
    assert_eq!(removed, 2);
    assert_eq!(cache.len(), 0);
    tracing::debug!(removed_count = removed, "Cleaned up expired entries");
    
    tracing::info!("Cache cleanup expired entries test passed");
    Ok(())
}

#[test]
fn test_cache_lru_eviction() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing LRU eviction behavior");
    
    let config = CommandCacheConfig::default().with_max_size(2);
    let mut cache = CommandCache::new(config);
    tracing::debug!(max_size = 2, "Created cache with small max size");

    let mut args1 = HashMap::new();
    args1.insert("param1".to_string(), json!("value1"));
    let mut args2 = HashMap::new();
    args2.insert("param2".to_string(), json!("value2"));
    let mut args3 = HashMap::new();
    args3.insert("param3".to_string(), json!("value3"));

    cache.insert("discord", "cmd1", &args1, json!("result1"), None);
    cache.insert("discord", "cmd2", &args2, json!("result2"), None);
    tracing::debug!(cache_len = 2, "Filled cache to capacity");

    assert_eq!(cache.len(), 2);

    // This should evict the least recently used entry (cmd1)
    cache.insert("discord", "cmd3", &args3, json!("result3"), None);
    tracing::debug!("Inserted third entry, should evict cmd1");

    assert_eq!(cache.len(), 2);
    assert!(cache.get("discord", "cmd1", &args1).is_none());
    assert!(cache.get("discord", "cmd2", &args2).is_some());
    assert!(cache.get("discord", "cmd3", &args3).is_some());
    tracing::debug!("Verified cmd1 evicted, cmd2 and cmd3 present");
    
    tracing::info!("LRU eviction test passed");
    Ok(())
}
