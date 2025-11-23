//! Discord platform implementation.

use crate::{
    ActorError, ActorErrorKind, Content, PlatformMetadata, PlatformMetadataBuilder,
    PlatformResult, PostId, ScheduleId, SocialMediaPlatform,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Discord platform implementation.
///
/// Integrates with Discord API for posting content to channels.
pub struct DiscordPlatform {
    /// Discord bot token for authentication.
    #[allow(dead_code)]
    token: String,
    /// Default channel ID for posting.
    channel_id: String,
}

impl DiscordPlatform {
    /// Create a new Discord platform instance.
    ///
    /// # Arguments
    ///
    /// * `token` - Discord bot token
    /// * `channel_id` - Default channel ID for posting
    ///
    /// # Errors
    ///
    /// Returns error if token or channel_id are empty.
    #[tracing::instrument(skip(token), fields(channel_id))]
    pub fn new(token: impl Into<String>, channel_id: impl Into<String>) -> PlatformResult<Self> {
        let token = token.into();
        let channel_id = channel_id.into();

        if token.is_empty() {
            return Err(ActorError::new(ActorErrorKind::AuthenticationFailed(
                "Discord token cannot be empty".to_string(),
            )));
        }

        if channel_id.is_empty() {
            return Err(ActorError::new(ActorErrorKind::InvalidConfiguration(
                "Discord channel_id cannot be empty".to_string(),
            )));
        }

        tracing::debug!("Created Discord platform instance");

        Ok(Self { token, channel_id })
    }

    /// Get the configured channel ID.
    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }
}

#[async_trait]
impl SocialMediaPlatform for DiscordPlatform {
    #[tracing::instrument(skip(self, content), fields(channel_id = %self.channel_id))]
    async fn post(&self, content: Content) -> PlatformResult<PostId> {
        tracing::debug!("Posting content to Discord");

        // Validate content
        if content.text().is_none() && content.media().is_empty() {
            return Err(ActorError::new(ActorErrorKind::ValidationFailed(
                "Content must have text or media".to_string(),
            )));
        }

        // Check text length limit
        if let Some(text) = content.text()
            && text.len() > 2000
        {
            return Err(ActorError::new(ActorErrorKind::ValidationFailed(
                format!("Text exceeds Discord limit of 2000 characters ({})", text.len()),
            )));
        }

        // Check media attachment limit
        if content.media().len() > 10 {
            return Err(ActorError::new(ActorErrorKind::ValidationFailed(
                format!(
                    "Too many media attachments ({}, max 10)",
                    content.media().len()
                ),
            )));
        }

        // In production, would use serenity or twilight to post via Discord API
        // For now, return a mock post ID
        tracing::info!("Content validated and ready for Discord posting");

        // Simulate successful post
        let post_id = format!("discord_msg_{}", Utc::now().timestamp());
        Ok(PostId(post_id))
    }

    #[tracing::instrument(skip(self, content), fields(channel_id = %self.channel_id, scheduled_time = %time))]
    async fn schedule(&self, content: Content, time: DateTime<Utc>) -> PlatformResult<ScheduleId> {
        tracing::debug!("Scheduling content for Discord");

        // Validate content first
        self.validate_content(&content)?;

        // Check that scheduled time is in the future
        let now = Utc::now();
        if time <= now {
            return Err(ActorError::new(ActorErrorKind::ValidationFailed(
                format!(
                    "Scheduled time must be in the future (now: {}, scheduled: {})",
                    now, time
                ),
            )));
        }

        tracing::info!(
            scheduled_time = %time,
            "Content scheduled for Discord posting"
        );

        // In production, would store schedule in database or use external scheduler
        let schedule_id = format!("discord_sched_{}", Utc::now().timestamp());
        Ok(ScheduleId(schedule_id))
    }

    #[tracing::instrument(skip(self), fields(post_id = %_id.0))]
    async fn delete_post(&self, _id: PostId) -> PlatformResult<()> {
        tracing::debug!("Deleting Discord post");

        // In production, would use Discord API to delete message
        tracing::info!("Discord post deleted");

        Ok(())
    }

    fn metadata(&self) -> PlatformMetadata {
        PlatformMetadataBuilder::default()
            .name("discord".to_string())
            .max_text_length(2000)
            .max_media_attachments(10)
            .supported_media_types(vec![
                "image".to_string(),
                "video".to_string(),
                "audio".to_string(),
            ])
            .build()
            .expect("Valid Discord metadata")
    }
}

impl DiscordPlatform {
    /// Validate content against Discord limits.
    fn validate_content(&self, content: &Content) -> PlatformResult<()> {
        if content.text().is_none() && content.media().is_empty() {
            return Err(ActorError::new(ActorErrorKind::ValidationFailed(
                "Content must have text or media".to_string(),
            )));
        }

        if let Some(text) = content.text()
            && text.len() > 2000
        {
            return Err(ActorError::new(ActorErrorKind::ValidationFailed(
                format!("Text exceeds Discord limit of 2000 characters ({})", text.len()),
            )));
        }

        if content.media().len() > 10 {
            return Err(ActorError::new(ActorErrorKind::ValidationFailed(
                format!(
                    "Too many media attachments ({}, max 10)",
                    content.media().len()
                ),
            )));
        }

        Ok(())
    }
}
