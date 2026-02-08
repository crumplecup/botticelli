//! Registry for MCP resources.

use crate::ResourceInfo;
use botticelli_error::McpResult;
use botticelli_interface::{McpResource, ReadParams};
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Registry for MCP resources.
#[derive(Clone, Default)]
pub struct ResourceRegistry {
    resources: Arc<
        Vec<Arc<dyn McpResource<Error = botticelli_error::McpError, ResourceInfo = ResourceInfo>>>,
    >,
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
    pub fn register(
        &mut self,
        resource: Arc<
            dyn McpResource<Error = botticelli_error::McpError, ResourceInfo = ResourceInfo>,
        >,
    ) {
        Arc::make_mut(&mut self.resources).push(resource);
    }

    /// Lists all resources.
    #[tracing::instrument(skip(self))]
    pub fn list(
        &self,
    ) -> Vec<Arc<dyn McpResource<Error = botticelli_error::McpError, ResourceInfo = ResourceInfo>>>
    {
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
                
                // Construct Parameters for the trait method
                let params = Parameters(ReadParams {
                    uri: uri.to_string(),
                });
                
                // Call trait method and extract content
                let result = resource.read(params).await
                    .map_err(|e| botticelli_error::McpError::execution_failed(e.message))?;
                
                return Ok(result.0.content);
            }
        }

        Err(botticelli_error::McpError::resource_not_found(format!(
            "No resource handler for URI: {}",
            uri
        )))
    }
}
