//! Tests for actor core functionality.

use async_trait::async_trait;
use botticelli_actor::{
    Actor, ActorConfigBuilder, ExecutionResult, PlatformMetadata, PlatformMetadataBuilder,
    PlatformResult, PostId, ScheduleId, Skill, SkillContext, SkillOutput, SkillRegistry,
    SkillResult, SocialMediaPlatform,
};
use serde_json::json;
use std::sync::Arc;

/// Mock platform for testing.
struct MockPlatform;

#[async_trait]
impl SocialMediaPlatform for MockPlatform {
    async fn post(
        &self,
        _content: botticelli_actor::Content,
    ) -> PlatformResult<PostId> {
        Ok(PostId("mock_post".to_string()))
    }

    async fn schedule(
        &self,
        _content: botticelli_actor::Content,
        _time: chrono::DateTime<chrono::Utc>,
    ) -> PlatformResult<ScheduleId> {
        Ok(ScheduleId("mock_schedule".to_string()))
    }

    async fn delete_post(&self, _id: PostId) -> PlatformResult<()> {
        Ok(())
    }

    fn metadata(&self) -> PlatformMetadata {
        PlatformMetadataBuilder::default()
            .name("mock".to_string())
            .max_text_length(280)
            .max_media_attachments(4)
            .supported_media_types(vec![])
            .build()
            .expect("Valid metadata")
    }
}

/// Mock skill for testing.
struct MockSkill {
    name: String,
    should_fail: bool,
}

#[async_trait]
impl Skill for MockSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Mock skill for testing"
    }

    async fn execute(&self, _context: &SkillContext) -> SkillResult<SkillOutput> {
        if self.should_fail {
            return Err(botticelli_actor::ActorError::new(
                botticelli_actor::ActorErrorKind::ValidationFailed("Mock failure".to_string()),
            ));
        }

        Ok(SkillOutput {
            skill_name: self.name.clone(),
            data: json!({"status": "success"}),
        })
    }
}

#[test]
fn test_actor_builder_missing_config() {
    let result = Actor::builder()
        .platform(Arc::new(MockPlatform))
        .build();

    assert!(result.is_err());
}

#[test]
fn test_actor_builder_missing_platform() {
    let config = ActorConfigBuilder::default()
        .name("Test Actor".to_string())
        .description("Test".to_string())
        .knowledge(vec![])
        .skills(vec![])
        .build()
        .expect("Valid config");

    let result = Actor::builder()
        .config(config)
        .build();

    assert!(result.is_err());
}

#[test]
fn test_actor_builder_success() {
    let config = ActorConfigBuilder::default()
        .name("Test Actor".to_string())
        .description("Test".to_string())
        .knowledge(vec![])
        .skills(vec![])
        .build()
        .expect("Valid config");

    let result = Actor::builder()
        .config(config)
        .platform(Arc::new(MockPlatform))
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_execution_result_new() {
    let result = ExecutionResult {
        succeeded: vec![],
        failed: vec![],
        skipped: vec![],
    };

    assert_eq!(result.succeeded.len(), 0);
    assert_eq!(result.failed.len(), 0);
    assert_eq!(result.skipped.len(), 0);
}
