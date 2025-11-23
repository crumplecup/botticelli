//! Tests for actor configuration.

use botticelli_actor::{
    ActorCacheConfig, ActorConfig, ActorSettings, CacheStrategy, ExecutionConfig, SkillConfig,
};
use std::collections::HashMap;

#[test]
fn test_actor_settings_defaults() {
    let settings = ActorSettings::default();
    assert_eq!(*settings.max_posts_per_day(), 10);
    assert_eq!(*settings.min_interval_minutes(), 60);
    assert_eq!(*settings.retry_attempts(), 3);
    assert_eq!(settings.timezone(), "UTC");
}

#[test]
fn test_actor_settings_builder() {
    let settings = ActorSettings::builder()
        .max_posts_per_day(20)
        .min_interval_minutes(30)
        .retry_attempts(5)
        .timezone("America/New_York".to_string())
        .build();

    assert_eq!(*settings.max_posts_per_day(), 20);
    assert_eq!(*settings.min_interval_minutes(), 30);
    assert_eq!(*settings.retry_attempts(), 5);
    assert_eq!(settings.timezone(), "America/New_York");
}

#[test]
fn test_cache_config_defaults() {
    let cache = ActorCacheConfig::default();
    assert_eq!(*cache.strategy(), CacheStrategy::Memory);
    assert_eq!(*cache.ttl_seconds(), 300);
    assert_eq!(*cache.max_entries(), 1000);
    assert_eq!(cache.disk_path(), &None);
}

#[test]
fn test_cache_config_builder() {
    let cache = ActorCacheConfig::builder()
        .strategy(CacheStrategy::Disk)
        .ttl_seconds(600)
        .max_entries(500)
        .disk_path(Some(".cache".into()))
        .build();

    assert_eq!(*cache.strategy(), CacheStrategy::Disk);
    assert_eq!(*cache.ttl_seconds(), 600);
    assert_eq!(*cache.max_entries(), 500);
    assert!(cache.disk_path().is_some());
}

#[test]
fn test_execution_config_defaults() {
    let exec = ExecutionConfig::default();
    assert!(*exec.stop_on_unrecoverable());
    assert_eq!(*exec.max_retries(), 3);
    assert!(*exec.continue_on_error());
}

#[test]
fn test_execution_config_builder() {
    let exec = ExecutionConfig::builder()
        .stop_on_unrecoverable(false)
        .max_retries(5)
        .continue_on_error(false)
        .build();

    assert!(!*exec.stop_on_unrecoverable());
    assert_eq!(*exec.max_retries(), 5);
    assert!(!*exec.continue_on_error());
}

#[test]
fn test_skill_config_defaults() {
    let skill = SkillConfig::default();
    assert!(*skill.enabled());
    assert!(skill.settings().is_empty());
}

#[test]
fn test_skill_config_builder() {
    let mut settings = HashMap::new();
    settings.insert("key".to_string(), serde_json::json!("value"));

    let skill = SkillConfig::builder()
        .enabled(false)
        .settings(settings.clone())
        .build();

    assert!(!*skill.enabled());
    assert_eq!(skill.settings().len(), 1);
}

#[test]
fn test_actor_config_from_file() {
    let config = ActorConfig::from_file("examples/actor.toml").expect("Failed to load config");

    assert_eq!(config.name(), "Post Scheduler");
    assert_eq!(config.knowledge().len(), 2);
    assert_eq!(config.skills().len(), 2);
    assert!(config.knowledge().contains(&"approved_posts_channel_1".to_string()));
    assert!(config.skills().contains(&"content_scheduling".to_string()));
}

#[test]
fn test_actor_config_validation() {
    let config = ActorConfig::builder()
        .name("Test Actor".to_string())
        .description("Test".to_string())
        .knowledge(vec![])
        .skills(vec![])
        .build();

    let warnings = config.validate();
    assert!(warnings.len() >= 2);
    assert!(warnings.iter().any(|w| w.contains("No knowledge tables")));
    assert!(warnings.iter().any(|w| w.contains("No skills configured")));
}

#[test]
fn test_actor_config_validation_valid() {
    let config = ActorConfig::builder()
        .name("Test Actor".to_string())
        .description("Test".to_string())
        .knowledge(vec!["table1".to_string()])
        .skills(vec!["skill1".to_string()])
        .build();

    let warnings = config.validate();
    assert_eq!(warnings.len(), 0);
}

#[test]
fn test_config_builder_with_defaults() {
    let config = ActorConfig::builder()
        .name("Minimal Actor".to_string())
        .description("A minimal configuration".to_string())
        .knowledge(vec!["table1".to_string()])
        .skills(vec!["skill1".to_string()])
        .build();

    assert_eq!(config.name(), "Minimal Actor");
    assert_eq!(*config.config().max_posts_per_day(), 10); // Default
    assert_eq!(*config.cache().strategy(), CacheStrategy::Memory); // Default
    assert_eq!(*config.execution().max_retries(), 3); // Default
}

#[test]
fn test_cache_strategy_serde() {
    let json_none = serde_json::to_string(&CacheStrategy::None).unwrap();
    assert_eq!(json_none, r#""none""#);

    let json_memory = serde_json::to_string(&CacheStrategy::Memory).unwrap();
    assert_eq!(json_memory, r#""memory""#);

    let json_disk = serde_json::to_string(&CacheStrategy::Disk).unwrap();
    assert_eq!(json_disk, r#""disk""#);
}
