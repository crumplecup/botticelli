//! Tests for built-in skills.

use async_trait::async_trait;
use botticelli_actor::{
    ContentSchedulingSkill, PlatformMetadata, PlatformMetadataBuilder, PlatformResult, PostId,
    RateLimitingSkill, ScheduleId, Skill, SkillContext, SocialMediaPlatform,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Mock platform for testing.
struct MockPlatform;

#[async_trait]
impl SocialMediaPlatform for MockPlatform {
    async fn post(&self, _content: botticelli_actor::Content) -> PlatformResult<PostId> {
        Ok(PostId("mock".to_string()))
    }

    async fn schedule(
        &self,
        _content: botticelli_actor::Content,
        _time: chrono::DateTime<chrono::Utc>,
    ) -> PlatformResult<ScheduleId> {
        Ok(ScheduleId("mock".to_string()))
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

#[tokio::test]
async fn test_content_scheduling_skill_defaults() {
    let skill = ContentSchedulingSkill::new();
    assert_eq!(skill.name(), "content_scheduling");

    let context = SkillContext {
        knowledge: HashMap::new(),
        config: HashMap::new(),
        platform: Arc::new(MockPlatform),
    };

    let result = skill.execute(&context).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output.skill_name, "content_scheduling");
    assert!(output.data.is_object());
}

#[tokio::test]
async fn test_content_scheduling_skill_with_config() {
    let skill = ContentSchedulingSkill::new();

    let mut config = HashMap::new();
    config.insert("schedule_window_start".to_string(), "10:00".to_string());
    config.insert("schedule_window_end".to_string(), "16:00".to_string());
    config.insert("randomize_within_window".to_string(), "false".to_string());

    let context = SkillContext {
        knowledge: HashMap::new(),
        config,
        platform: Arc::new(MockPlatform),
    };

    let result = skill.execute(&context).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    let data = &output.data;
    assert_eq!(data["window_start"], "10:00");
    assert_eq!(data["window_end"], "16:00");
    assert_eq!(data["randomized"], false);
}

#[tokio::test]
async fn test_rate_limiting_skill_defaults() {
    let skill = RateLimitingSkill::new();
    assert_eq!(skill.name(), "rate_limiting");

    let context = SkillContext {
        knowledge: HashMap::new(),
        config: HashMap::new(),
        platform: Arc::new(MockPlatform),
    };

    let result = skill.execute(&context).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output.skill_name, "rate_limiting");
    assert!(output.data.is_object());
}

#[tokio::test]
async fn test_rate_limiting_skill_with_config() {
    let skill = RateLimitingSkill::new();

    let mut config = HashMap::new();
    config.insert("max_posts_per_day".to_string(), "20".to_string());
    config.insert("min_interval_minutes".to_string(), "30".to_string());

    let context = SkillContext {
        knowledge: HashMap::new(),
        config,
        platform: Arc::new(MockPlatform),
    };

    let result = skill.execute(&context).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    let data = &output.data;
    assert_eq!(data["max_posts_per_day"], 20);
    assert_eq!(data["min_interval_minutes"], 30);
    assert_eq!(data["can_post"], true);
}

#[tokio::test]
async fn test_rate_limiting_skill_remaining_count() {
    let skill = RateLimitingSkill::new();

    let mut config = HashMap::new();
    config.insert("max_posts_per_day".to_string(), "10".to_string());

    let context = SkillContext {
        knowledge: HashMap::new(),
        config,
        platform: Arc::new(MockPlatform),
    };

    let result = skill.execute(&context).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    let data = &output.data;
    // With current_count=0 (mocked), remaining should equal max
    assert_eq!(data["remaining"], 10);
}
