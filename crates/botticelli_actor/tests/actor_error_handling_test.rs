//! Tests for actor error handling and recovery behavior.

use async_trait::async_trait;
use botticelli_actor::{
    ActorConfigBuilder, ActorError, ActorErrorKind, ExecutionConfigBuilder, Platform,
    PlatformCapability, PlatformMessage, PlatformMetadata, Skill, SkillContext, SkillInfoBuilder,
    SkillOutput, SkillOutputBuilder,
};
use std::sync::atomic::{AtomicU32, Ordering};

/// Mock platform for testing.
#[allow(dead_code)]
struct MockPlatform;

#[async_trait]
impl Platform for MockPlatform {
    async fn post(&self, _message: &PlatformMessage) -> Result<PlatformMetadata, ActorError> {
        Ok(PlatformMetadata::new())
    }

    async fn verify_connection(&self) -> Result<(), ActorError> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<PlatformCapability> {
        vec![PlatformCapability::Text]
    }

    fn platform_name(&self) -> &str {
        "mock"
    }
}

/// Skill that fails with recoverable errors for a certain number of attempts.
#[allow(dead_code)]
struct RecoverableErrorSkill {
    attempts: AtomicU32,
    fail_count: u32,
}

impl RecoverableErrorSkill {
    #[allow(dead_code)]
    fn new(fail_count: u32) -> Self {
        Self {
            attempts: AtomicU32::new(0),
            fail_count,
        }
    }
}

#[async_trait]
impl Skill for RecoverableErrorSkill {
    fn name(&self) -> &str {
        "recoverable_error"
    }

    fn description(&self) -> &str {
        "Skill that fails with recoverable errors"
    }

    async fn execute(&self, _context: &SkillContext) -> Result<SkillOutput, ActorError> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);

        if attempt < self.fail_count {
            // Return recoverable error
            Err(ActorError::new(ActorErrorKind::PlatformTemporary(format!(
                "Temporary failure (attempt {})",
                attempt
            ))))
        } else {
            // Success after retries
            Ok(SkillOutputBuilder::default()
                .skill_name(self.name())
                .data(serde_json::json!({"attempts": attempt + 1}))
                .build()
                .expect("Valid skill output"))
        }
    }
}

/// Skill that always fails with unrecoverable errors.
#[allow(dead_code)]
struct UnrecoverableErrorSkill;

#[async_trait]
impl Skill for UnrecoverableErrorSkill {
    fn name(&self) -> &str {
        "unrecoverable_error"
    }

    fn description(&self) -> &str {
        "Skill that always fails with unrecoverable errors"
    }

    async fn execute(&self, _context: &SkillContext) -> Result<SkillOutput, ActorError> {
        Err(ActorError::new(ActorErrorKind::AuthenticationFailed(
            "Authentication failed".to_string(),
        )))
    }
}

/// Skill that succeeds immediately.
#[allow(dead_code)]
struct SuccessSkill;

#[async_trait]
impl Skill for SuccessSkill {
    fn name(&self) -> &str {
        "success"
    }

    fn description(&self) -> &str {
        "Skill that always succeeds"
    }

    async fn execute(&self, _context: &SkillContext) -> Result<SkillOutput, ActorError> {
        Ok(SkillOutputBuilder::default()
            .skill_name(self.name())
            .data(serde_json::json!({"status": "success"}))
            .build()
            .expect("Valid skill output"))
    }
}

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

    assert_eq!(info.name, "test_skill");
    assert_eq!(info.description, "A test skill");
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
