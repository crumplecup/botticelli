//! No-op platform implementation for actors that don't need posting.

use crate::{ActorResult, Platform, PlatformCapability, PlatformMessage, PlatformMetadata};
use async_trait::async_trait;
use tracing::debug;

/// Platform implementation that does nothing.
///
/// Used for actors that execute narratives or skills without posting to any
/// social media platform (e.g., content generation, curation).
#[derive(Debug, Clone, Default)]
pub struct NoOpPlatform;

impl NoOpPlatform {
    /// Create a new no-op platform.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Platform for NoOpPlatform {
    async fn post(&self, message: &PlatformMessage) -> ActorResult<PlatformMetadata> {
        debug!(
            text_len = message.text.len(),
            media_count = message.media_urls.len(),
            "NoOpPlatform: post() called (no action taken)"
        );
        Ok(PlatformMetadata::new())
    }

    async fn verify_connection(&self) -> ActorResult<()> {
        debug!("NoOpPlatform: verify_connection() called (always succeeds)");
        Ok(())
    }

    fn capabilities(&self) -> Vec<PlatformCapability> {
        vec![]
    }

    fn platform_name(&self) -> &str {
        "noop"
    }
}
