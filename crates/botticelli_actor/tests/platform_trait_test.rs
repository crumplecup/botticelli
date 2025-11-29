//! Tests for platform trait and basic implementations.

#[cfg(feature = "discord")]
use botticelli_actor::{DiscordPlatform, Platform, PlatformCapability, PlatformMessage};

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_platform_creation() {
    let platform = DiscordPlatform::new("123456789").expect("Should create platform");

    assert_eq!(platform.platform_name(), "discord");
    assert_eq!(platform.channel_id(), "123456789");
}

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_platform_capabilities() {
    let platform = DiscordPlatform::new("123456789").expect("Should create platform");

    let caps = platform.capabilities();
    assert!(caps.contains(&PlatformCapability::Text));
    assert!(caps.contains(&PlatformCapability::Images));
    assert!(caps.contains(&PlatformCapability::Videos));
}

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_platform_post_validation() {
    let platform = DiscordPlatform::new("123456789").expect("Should create platform");

    // Empty message should fail
    let message = PlatformMessage {
        text: String::new(),
        media_urls: vec![],
    };

    let result = platform.post(&message).await;
    assert!(result.is_err());
}

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_platform_text_limit() {
    let platform = DiscordPlatform::new("123456789").expect("Should create platform");

    // Text too long should fail
    let long_text = "a".repeat(2001);
    let message = PlatformMessage {
        text: long_text,
        media_urls: vec![],
    };

    let result = platform.post(&message).await;
    assert!(result.is_err());
}

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_platform_valid_post() {
    let platform = DiscordPlatform::new("123456789").expect("Should create platform");

    let message = PlatformMessage {
        text: "Hello Discord!".to_string(),
        media_urls: vec![],
    };

    let result = platform.post(&message).await;
    assert!(result.is_ok());

    let metadata = result.unwrap();
    assert!(metadata.contains_key("channel_id"));
    assert!(metadata.contains_key("message_id"));
}
