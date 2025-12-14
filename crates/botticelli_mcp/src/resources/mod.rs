//! MCP resource system.
//!
//! Resources are data sources that LLMs can read. They follow URI patterns like:
//! - `content://approved_discord_posts/123` - Content by ID
//! - `narrative://curate_content` - Narrative TOML file

use async_trait::async_trait;
use botticelli_error::McpResult;
use std::sync::Arc;
use tracing::{debug, instrument};

#[cfg(feature = "database")]
mod content;
mod narrative;

#[cfg(feature = "database")]
pub use content::ContentResource;
pub use narrative::NarrativeResource;

/// MCP resource that LLMs can read.
#[async_trait]
pub trait McpResource: Send + Sync {
    /// URI pattern this resource handles (e.g., "content://", "narrative://")
    fn uri_pattern(&self) -> &'static str;

    /// Resource description for LLM
    fn description(&self) -> &'static str;

    /// Check if this resource handles the given URI
    fn matches(&self, uri: &str) -> bool {
        uri.starts_with(self.uri_pattern())
    }

    /// Read resource content
    async fn read(&self, uri: &str) -> McpResult<String>;

    /// List available resources (optional)
    async fn list(&self) -> McpResult<Vec<ResourceInfo>> {
        Ok(vec![])
    }
}

/// Information about a resource.
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    pub description: String,
    /// MIME type (optional)
    pub mime_type: Option<String>,
}

/// Registry for MCP resources.
#[derive(Clone, Default)]
pub struct ResourceRegistry {
    resources: Arc<Vec<Arc<dyn McpResource>>>,
}

impl ResourceRegistry {
    /// Creates a new resource registry.
    pub fn new() -> Self {
        Self {
            resources: Arc::new(vec![]),
        }
    }

    /// Registers a resource.
    pub fn register(&mut self, resource: Arc<dyn McpResource>) {
        Arc::make_mut(&mut self.resources).push(resource);
    }

    /// Lists all resources.
    pub fn list(&self) -> Vec<Arc<dyn McpResource>> {
        self.resources.as_ref().clone()
    }

    /// Lists all available resource instances.
    #[instrument(skip(self))]
    pub async fn list_all(&self) -> McpResult<Vec<ResourceInfo>> {
        let mut all_resources = Vec::new();

        for resource in self.resources.as_ref() {
            match resource.list().await {
                Ok(resources) => all_resources.extend(resources),
                Err(e) => {
                    debug!(error = %e, pattern = %resource.uri_pattern(), "Failed to list resources")
                }
            }
        }

        Ok(all_resources)
    }

    /// Reads a resource by URI.
    #[instrument(skip(self), fields(uri))]
    pub async fn read(&self, uri: &str) -> McpResult<String> {
        for resource in self.resources.as_ref() {
            if resource.matches(uri) {
                debug!(uri, pattern = %resource.uri_pattern(), "Resource matched");
                return resource.read(uri).await;
            }
        }

        Err(botticelli_error::McpError::resource_not_found(format!(
            "No resource handler for URI: {}",
            uri
        )))
    }
}
