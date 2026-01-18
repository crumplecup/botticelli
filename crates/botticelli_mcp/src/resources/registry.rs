//! Registry for MCP resources.

use crate::ResourceInfo;
use botticelli_error::McpResult;
use botticelli_interface::McpResource;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Registry for MCP resources.
#[derive(Clone, Default)]
pub struct ResourceRegistry {
    resources: Arc<Vec<Arc<dyn McpResource<Error = botticelli_error::McpError, ResourceInfo = ResourceInfo>>>>,
}

impl ResourceRegistry {
    /// Creates a new resource registry.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            resources: Arc::new(vec![]),
        }
    }

    /// Registers a resource.
    #[tracing::instrument(skip(self, resource))]
    pub fn register(&mut self, resource: Arc<dyn McpResource<Error = botticelli_error::McpError, ResourceInfo = ResourceInfo>>) {
        Arc::make_mut(&mut self.resources).push(resource);
    }

    /// Lists all resources.
    #[tracing::instrument(skip(self))]
    pub fn list(&self) -> Vec<Arc<dyn McpResource<Error = botticelli_error::McpError, ResourceInfo = ResourceInfo>>> {
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
