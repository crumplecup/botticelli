//! Tests for Discord platform implementation.

#[cfg(feature = "discord")]
mod discord_tests {
    use botticelli_actor::{
        platforms::DiscordPlatform, ContentBuilder, MediaAttachmentBuilder, MediaType,
        SocialMediaPlatform,
    };
    use chrono::Utc;

    #[tokio::test]
    async fn test_discord_platform_new() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        assert_eq!(platform.channel_id(), "123456789");
    }

    #[test]
    fn test_discord_platform_new_empty_token() {
        let result = DiscordPlatform::new("", "123456789");
        assert!(result.is_err());
    }

    #[test]
    fn test_discord_platform_new_empty_channel() {
        let result = DiscordPlatform::new("test_token", "");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_discord_platform_post_text_only() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        
        let content = ContentBuilder::default()
            .text(Some("Test message".to_string()))
            .build()
            .expect("Valid content");

        let result = platform.post(content).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discord_platform_post_empty_content() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        
        let content = ContentBuilder::default()
            .build()
            .expect("Valid content");

        let result = platform.post(content).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_discord_platform_post_text_too_long() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        
        let long_text = "a".repeat(2001);
        let content = ContentBuilder::default()
            .text(Some(long_text))
            .build()
            .expect("Valid content");

        let result = platform.post(content).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_discord_platform_post_with_media() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        
        let attachment = MediaAttachmentBuilder::default()
            .url("https://example.com/image.png".to_string())
            .media_type(MediaType::Image)
            .build()
            .expect("Valid attachment");

        let content = ContentBuilder::default()
            .text(Some("Check this out!".to_string()))
            .media(vec![attachment])
            .build()
            .expect("Valid content");

        let result = platform.post(content).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discord_platform_post_too_many_attachments() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        
        let mut attachments = vec![];
        for i in 0..11 {
            attachments.push(
                MediaAttachmentBuilder::default()
                    .url(format!("https://example.com/image{}.png", i))
                    .media_type(MediaType::Image)
                    .build()
                    .expect("Valid attachment"),
            );
        }

        let content = ContentBuilder::default()
            .media(attachments)
            .build()
            .expect("Valid content");

        let result = platform.post(content).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_discord_platform_schedule_future() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        
        let content = ContentBuilder::default()
            .text(Some("Scheduled message".to_string()))
            .build()
            .expect("Valid content");

        let future_time = Utc::now() + chrono::Duration::hours(1);
        let result = platform.schedule(content, future_time).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discord_platform_schedule_past() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        
        let content = ContentBuilder::default()
            .text(Some("Scheduled message".to_string()))
            .build()
            .expect("Valid content");

        let past_time = Utc::now() - chrono::Duration::hours(1);
        let result = platform.schedule(content, past_time).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_discord_platform_delete() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        
        let post_id = botticelli_actor::PostId("test_message_123".to_string());
        let result = platform.delete_post(post_id).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_discord_platform_metadata() {
        let platform = DiscordPlatform::new("test_token", "123456789").expect("Valid platform");
        let metadata = platform.metadata();

        assert_eq!(metadata.name(), "discord");
        assert_eq!(*metadata.max_text_length(), 2000);
        assert_eq!(*metadata.max_media_attachments(), 10);
        assert_eq!(metadata.supported_media_types().len(), 3);
    }
}
