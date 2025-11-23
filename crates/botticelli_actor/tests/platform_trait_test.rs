//! Tests for platform trait with mock implementation.

use async_trait::async_trait;
use botticelli_actor::{
    Content, ContentBuilder, MediaAttachmentBuilder, MediaType, PlatformMetadata,
    PlatformMetadataBuilder, PlatformResult, PostId, ScheduleId, SocialMediaPlatform,
};
use chrono::{DateTime, Utc};

/// Mock platform for testing.
struct MockPlatform {
    name: String,
    fail_post: bool,
}

impl MockPlatform {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fail_post: false,
        }
    }

    fn with_post_failure(mut self) -> Self {
        self.fail_post = true;
        self
    }
}

#[async_trait]
impl SocialMediaPlatform for MockPlatform {
    async fn post(&self, _content: Content) -> PlatformResult<PostId> {
        if self.fail_post {
            return Err(botticelli_actor::ActorError::new(
                botticelli_actor::ActorErrorKind::PlatformTemporary("Mock failure".to_string()),
            ));
        }
        Ok(PostId("mock_post_123".to_string()))
    }

    async fn schedule(&self, _content: Content, _time: DateTime<Utc>) -> PlatformResult<ScheduleId> {
        Ok(ScheduleId("mock_schedule_456".to_string()))
    }

    async fn delete_post(&self, _id: PostId) -> PlatformResult<()> {
        Ok(())
    }

    fn metadata(&self) -> PlatformMetadata {
        PlatformMetadataBuilder::default()
            .name(self.name.clone())
            .max_text_length(280)
            .max_media_attachments(4)
            .supported_media_types(vec!["image".to_string(), "video".to_string()])
            .build()
            .expect("Valid metadata")
    }
}

#[tokio::test]
async fn test_mock_platform_post_success() {
    let platform = MockPlatform::new("test");
    let content = ContentBuilder::default()
        .text(Some("Test post".to_string()))
        .build()
        .expect("Valid content");

    let result = platform.post(content).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, "mock_post_123");
}

#[tokio::test]
async fn test_mock_platform_post_failure() {
    let platform = MockPlatform::new("test").with_post_failure();
    let content = ContentBuilder::default()
        .text(Some("Test post".to_string()))
        .build()
        .expect("Valid content");

    let result = platform.post(content).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().is_recoverable());
}

#[tokio::test]
async fn test_mock_platform_schedule() {
    let platform = MockPlatform::new("test");
    let content = ContentBuilder::default()
        .text(Some("Scheduled post".to_string()))
        .build()
        .expect("Valid content");
    let time = Utc::now();

    let result = platform.schedule(content, time).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, "mock_schedule_456");
}

#[tokio::test]
async fn test_mock_platform_delete() {
    let platform = MockPlatform::new("test");
    let post_id = PostId("test_123".to_string());

    let result = platform.delete_post(post_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mock_platform_metadata() {
    let platform = MockPlatform::new("test_platform");
    let metadata = platform.metadata();

    assert_eq!(metadata.name(), "test_platform");
    assert_eq!(*metadata.max_text_length(), 280);
    assert_eq!(*metadata.max_media_attachments(), 4);
    assert_eq!(metadata.supported_media_types().len(), 2);
}

#[test]
fn test_content_builder() {
    let attachment = MediaAttachmentBuilder::default()
        .url("https://example.com/image.png".to_string())
        .media_type(MediaType::Image)
        .alt_text(Some("Test image".to_string()))
        .build()
        .expect("Valid attachment");

    let content = ContentBuilder::default()
        .text(Some("Hello world".to_string()))
        .media(vec![attachment])
        .build()
        .expect("Valid content");

    assert_eq!(content.text(), &Some("Hello world".to_string()));
    assert_eq!(content.media().len(), 1);
    assert_eq!(content.media()[0].url(), "https://example.com/image.png");
    assert_eq!(content.media()[0].media_type(), &MediaType::Image);
}

#[test]
fn test_media_attachment_builder() {
    let attachment = MediaAttachmentBuilder::default()
        .url("https://example.com/video.mp4".to_string())
        .media_type(MediaType::Video)
        .build()
        .expect("Valid attachment");

    assert_eq!(attachment.url(), "https://example.com/video.mp4");
    assert_eq!(attachment.media_type(), &MediaType::Video);
    assert_eq!(attachment.alt_text(), &None);
}
