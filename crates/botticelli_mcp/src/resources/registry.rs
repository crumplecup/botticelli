//! Registry for MCP resources.

use async_trait::async_trait;
use crate::ResourceInfo;
use botticelli_error::McpResult;
use botticelli_interface::{McpResource, ReadParams, ReadResult};
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
}

/// ResourceRegistry implements McpResource for tool-native MCP integration.
///
/// This allows the registry itself to be exposed as an MCP tool via
/// `#[elicit_trait_tools_router]`. The registry routes requests to the
/// appropriate registered resource based on URI pattern matching.
#[async_trait]
impl McpResource for ResourceRegistry {
    type Error = botticelli_error::McpError;
    type ResourceInfo = ResourceInfo;

    fn uri_pattern(&self) -> &'static str {
        "*"  // Registry matches all patterns, delegates to sub-resources
    }

    fn description(&self) -> &'static str {
        "Registry of all available MCP resources"
    }

    /// Read resource content by URI (tool-native signature).
    ///
    /// Routes the request to the appropriate registered resource based on URI pattern.
    #[instrument(skip(self), fields(uri = %params.0.uri))]
    async fn read(
        &self,
        params: Parameters<ReadParams>,
    ) -> Result<rmcp::Json<ReadResult>, rmcp::ErrorData> {
        let uri = &params.0.uri;

        for resource in self.resources.as_ref() {
            if resource.matches(uri) {
                debug!(uri, pattern = %resource.uri_pattern(), "Resource matched");
                return resource.read(params).await;
            }
        }

        Err(rmcp::ErrorData::internal_error(
            format!("No resource handler for URI: {}", uri),
            None,
        ))
    }

    /// List all available resources from all registered resources.
    #[instrument(skip(self))]
    async fn list(&self) -> Result<Vec<Self::ResourceInfo>, Self::Error> {
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
}
