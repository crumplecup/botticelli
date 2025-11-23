//! Social media platform trait and types.

use crate::{ActorError, Content};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// Result type for platform operations.
pub type PlatformResult<T> = Result<T, ActorError>;

/// Platform-specific post identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display)]
#[display("{}", _0)]
pub struct PostId(pub String);

/// Platform-specific schedule identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display)]
#[display("{}", _0)]
pub struct ScheduleId(pub String);

/// Metadata about a platform.
#[derive(Debug, Clone, PartialEq, Eq, Getters, Serialize, Deserialize, TypedBuilder)]
pub struct PlatformMetadata {
    /// Platform name (e.g., "discord", "twitter").
    name: String,

    /// Maximum text length for posts.
    max_text_length: usize,

    /// Maximum number of media attachments.
    max_media_attachments: usize,

    /// Supported media types.
    supported_media_types: Vec<String>,
}

/// Trait for social media platform implementations.
#[async_trait]
pub trait SocialMediaPlatform: Send + Sync {
    /// Post content immediately.
    ///
    /// # Arguments
    ///
    /// * `content` - Content to post
    ///
    /// # Returns
    ///
    /// Platform-specific post ID on success.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Authentication fails
    /// - Content validation fails
    /// - Rate limit exceeded
    /// - Platform API error
    async fn post(&self, content: Content) -> PlatformResult<PostId>;

    /// Schedule content for future posting.
    ///
    /// # Arguments
    ///
    /// * `content` - Content to post
    /// * `time` - When to post the content
    ///
    /// # Returns
    ///
    /// Platform-specific schedule ID on success.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Authentication fails
    /// - Content validation fails
    /// - Scheduling not supported
    /// - Platform API error
    async fn schedule(&self, content: Content, time: DateTime<Utc>) -> PlatformResult<ScheduleId>;

    /// Delete a post.
    ///
    /// # Arguments
    ///
    /// * `id` - Post ID to delete
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Authentication fails
    /// - Post not found
    /// - Insufficient permissions
    /// - Platform API error
    async fn delete_post(&self, id: PostId) -> PlatformResult<()>;

    /// Get platform-specific metadata.
    ///
    /// Returns information about platform constraints and capabilities.
    fn metadata(&self) -> PlatformMetadata;
}
