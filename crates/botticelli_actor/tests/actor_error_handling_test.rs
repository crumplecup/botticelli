//! Tests for actor error handling and recovery behavior.

use botticelli_actor::{
    ActorConfigBuilder, ActorError, ActorErrorKind, ExecutionConfigBuilder, SkillInfoBuilder,
};

#[test]
fn test_recoverable_error_classification() {
    // Test that error kinds are correctly classified as recoverable
    let recoverable = ActorError::new(ActorErrorKind::PlatformTemporary("test".to_string()));
    assert!(recoverable.is_recoverable());

    let rate_limit = ActorError::new(ActorErrorKind::RateLimitExceeded(60));
    assert!(rate_limit.is_recoverable());

    let validation = ActorError::new(ActorErrorKind::ValidationFailed("test".to_string()));
    assert!(validation.is_recoverable());

    let resource = ActorError::new(ActorErrorKind::ResourceUnavailable("test".to_string()));
    assert!(resource.is_recoverable());
}

#[test]
fn test_unrecoverable_error_classification() {
    // Test that error kinds are correctly classified as unrecoverable
    let auth = ActorError::new(ActorErrorKind::AuthenticationFailed("test".to_string()));
    assert!(!auth.is_recoverable());

    let config = ActorError::new(ActorErrorKind::InvalidConfiguration("test".to_string()));
    assert!(!config.is_recoverable());

    let platform = ActorError::new(ActorErrorKind::PlatformPermanent("test".to_string()));
    assert!(!platform.is_recoverable());

    let db = ActorError::new(ActorErrorKind::DatabaseFailed("test".to_string()));
    assert!(!db.is_recoverable());
}

#[test]
fn test_skill_info_builder() {
    // Test that SkillInfo can be built properly
    let info = SkillInfoBuilder::default()
        .name("test_skill")
        .description("A test skill")
        .build()
        .expect("Valid skill info");

    assert_eq!(info.name(), "test_skill");
    assert_eq!(info.description(), "A test skill");
}

#[test]
fn test_config_validation_with_high_retries() {
    // Test that configuration validation warns about high retry counts
    let config = ActorConfigBuilder::default()
        .name("test".to_string())
        .description("Test actor".to_string())
        .knowledge(vec!["test_table".to_string()])
        .skills(vec!["test_skill".to_string()])
        .execution(
            ExecutionConfigBuilder::default()
                .max_retries(100) // Very high retry count
                .build()
                .expect("Valid execution config"),
        )
        .build()
        .expect("Valid actor config");

    let warnings = config.validate();

    // Should have warning about high retry count
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("max_retries") && w.contains("very high"))
    );
}

#[test]
fn test_config_validation_with_zero_cache_ttl() {
    // Test that configuration validation warns about zero cache TTL
    let config = ActorConfigBuilder::default()
        .name("test".to_string())
        .description("Test actor".to_string())
        .knowledge(vec!["test_table".to_string()])
        .skills(vec!["test_skill".to_string()])
        .cache(
            botticelli_actor::ActorCacheConfigBuilder::default()
                .ttl_seconds(0) // Zero TTL
                .build()
                .expect("Valid cache config"),
        )
        .build()
        .expect("Valid actor config");

    let warnings = config.validate();

    // Should have warning about zero TTL
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("ttl_seconds") && w.contains("expire immediately"))
    );
}

#[test]
fn test_config_validation_with_excessive_cache_size() {
    // Test that configuration validation warns about excessive cache size
    let config = ActorConfigBuilder::default()
        .name("test".to_string())
        .description("Test actor".to_string())
        .knowledge(vec!["test_table".to_string()])
        .skills(vec!["test_skill".to_string()])
        .cache(
            botticelli_actor::ActorCacheConfigBuilder::default()
                .max_entries(200000) // Very large
                .build()
                .expect("Valid cache config"),
        )
        .build()
        .expect("Valid actor config");

    let warnings = config.validate();

    // Should have warning about large cache size
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("max_entries") && w.contains("very large"))
    );
}
