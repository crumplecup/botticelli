//! Tests for skill registry.

use async_trait::async_trait;
use botticelli_actor::{Skill, SkillContext, SkillOutput, SkillRegistry, SkillResult};
use serde_json::json;

/// Mock skill for testing.
struct MockSkill {
    name: String,
    description: String,
    fail: bool,
}

impl MockSkill {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            fail: false,
        }
    }

    fn with_failure(mut self) -> Self {
        self.fail = true;
        self
    }
}

#[async_trait]
impl Skill for MockSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(&self, _context: &SkillContext) -> SkillResult<SkillOutput> {
        if self.fail {
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
fn test_skill_registry_new() {
    let registry = SkillRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[test]
fn test_skill_registry_register() {
    let mut registry = SkillRegistry::new();
    let skill = MockSkill::new("test_skill", "A test skill");

    registry.register(std::sync::Arc::new(skill));
    assert_eq!(registry.len(), 1);
    assert!(!registry.is_empty());
}

#[test]
fn test_skill_registry_get() {
    let mut registry = SkillRegistry::new();
    let skill = MockSkill::new("test_skill", "A test skill");

    registry.register(std::sync::Arc::new(skill));

    let retrieved = registry.get("test_skill");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name(), "test_skill");

    let not_found = registry.get("nonexistent");
    assert!(not_found.is_none());
}

#[test]
fn test_skill_registry_list() {
    let mut registry = SkillRegistry::new();

    registry.register(std::sync::Arc::new(MockSkill::new(
        "skill1",
        "First skill",
    )));
    registry.register(std::sync::Arc::new(MockSkill::new(
        "skill2",
        "Second skill",
    )));
    registry.register(std::sync::Arc::new(MockSkill::new(
        "skill3",
        "Third skill",
    )));

    let list = registry.list();
    assert_eq!(list.len(), 3);

    let names: Vec<_> = list.iter().map(|info| info.name.as_str()).collect();
    assert!(names.contains(&"skill1"));
    assert!(names.contains(&"skill2"));
    assert!(names.contains(&"skill3"));
}

#[tokio::test]
async fn test_skill_registry_execute_success() {
    use botticelli_actor::SocialMediaPlatform;
    use std::collections::HashMap;
    use std::sync::Arc;

    struct DummyPlatform;

    #[async_trait]
    impl SocialMediaPlatform for DummyPlatform {
        async fn post(
            &self,
            _content: botticelli_actor::Content,
        ) -> botticelli_actor::PlatformResult<botticelli_actor::PostId> {
            Ok(botticelli_actor::PostId("dummy".to_string()))
        }

        async fn schedule(
            &self,
            _content: botticelli_actor::Content,
            _time: chrono::DateTime<chrono::Utc>,
        ) -> botticelli_actor::PlatformResult<botticelli_actor::ScheduleId> {
            Ok(botticelli_actor::ScheduleId("dummy".to_string()))
        }

        async fn delete_post(
            &self,
            _id: botticelli_actor::PostId,
        ) -> botticelli_actor::PlatformResult<()> {
            Ok(())
        }

        fn metadata(&self) -> botticelli_actor::PlatformMetadata {
            botticelli_actor::PlatformMetadata::builder()
                .name("dummy".to_string())
                .max_text_length(280)
                .max_media_attachments(4)
                .supported_media_types(vec![])
                .build()
        }
    }

    let mut registry = SkillRegistry::new();
    let skill = MockSkill::new("test_skill", "A test skill");
    registry.register(Arc::new(skill));

    let context = SkillContext {
        knowledge: HashMap::new(),
        config: HashMap::new(),
        platform: Arc::new(DummyPlatform),
    };

    let result = registry.execute("test_skill", &context).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output.skill_name, "test_skill");
    assert_eq!(output.data, json!({"status": "success"}));
}

#[tokio::test]
async fn test_skill_registry_execute_not_found() {
    use std::collections::HashMap;
    use std::sync::Arc;

    struct DummyPlatform;

    #[async_trait]
    impl botticelli_actor::SocialMediaPlatform for DummyPlatform {
        async fn post(
            &self,
            _content: botticelli_actor::Content,
        ) -> botticelli_actor::PlatformResult<botticelli_actor::PostId> {
            Ok(botticelli_actor::PostId("dummy".to_string()))
        }

        async fn schedule(
            &self,
            _content: botticelli_actor::Content,
            _time: chrono::DateTime<chrono::Utc>,
        ) -> botticelli_actor::PlatformResult<botticelli_actor::ScheduleId> {
            Ok(botticelli_actor::ScheduleId("dummy".to_string()))
        }

        async fn delete_post(
            &self,
            _id: botticelli_actor::PostId,
        ) -> botticelli_actor::PlatformResult<()> {
            Ok(())
        }

        fn metadata(&self) -> botticelli_actor::PlatformMetadata {
            botticelli_actor::PlatformMetadata::builder()
                .name("dummy".to_string())
                .max_text_length(280)
                .max_media_attachments(4)
                .supported_media_types(vec![])
                .build()
        }
    }

    let registry = SkillRegistry::new();

    let context = SkillContext {
        knowledge: HashMap::new(),
        config: HashMap::new(),
        platform: Arc::new(DummyPlatform),
    };

    let result = registry.execute("nonexistent", &context).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(!err.is_recoverable());
}
