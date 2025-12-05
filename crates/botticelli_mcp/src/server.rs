//! MCP server implementation.

use crate::{tools::ToolRegistry, PrometheusMetrics, ResourceRegistry};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;
use mcp_spec::{
    content::Content,
    handler::{PromptError, ResourceError, ToolError},
    prompt::Prompt,
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// MCP server for Botticelli implementing the Router trait.
#[derive(Clone)]
pub struct BotticelliRouter {
    name: String,
    version: String,
    tools: ToolRegistry,
    resources: ResourceRegistry,
    metrics: Arc<PrometheusMetrics>,
}

impl BotticelliRouter {
    /// Creates a new router builder.
    pub fn builder() -> BotticelliRouterBuilder {
        BotticelliRouterBuilder::default()
    }

    /// Gets the metrics collector.
    pub fn metrics(&self) -> &Arc<PrometheusMetrics> {
        &self.metrics
    }
}

impl Router for BotticelliRouter {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn instructions(&self) -> String {
        format!(
            "Botticelli MCP Server v{}\n\n\
            This server provides tools for interacting with the Botticelli LLM orchestration platform. \
            You can query databases, execute narratives, and interact with social media through these tools.\n\n\
            Available tools: {}",
            self.version,
            self.tools
                .list()
                .iter()
                .map(|t| t.name())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false) // tools don't change dynamically (yet)
            .with_resources(false, false) // resources don't change dynamically (yet)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools
            .list()
            .iter()
            .map(|tool| {
                Tool::new(
                    tool.name().to_string(),
                    tool.description().to_string(),
                    tool.input_schema(),
                )
            })
            .collect()
    }

    #[instrument(skip(self, arguments), fields(tool = %tool_name))]
    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        debug!(tool = %tool_name, args = ?arguments, "Tool called");

        let tools = self.tools.clone();
        let tool_name = tool_name.to_string();

        Box::pin(async move {
            match tools.execute(&tool_name, arguments).await {
                Ok(result) => {
                    info!(tool = %tool_name, "Tool executed successfully");
                    // Convert JSON result to Content
                    let text = serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string());
                    Ok(vec![Content::text(text)])
                }
                Err(e) => {
                    debug!(tool = %tool_name, error = %e, "Tool execution failed");
                    Err(ToolError::ExecutionError(e.to_string()))
                }
            }
        })
    }

    #[instrument(skip(self))]
    fn list_resources(&self) -> Vec<Resource> {
        // For Phase 2, return empty list as resource listing requires async context
        // Resources are still readable via read_resource()
        // TODO: Pre-compute resource list during server initialization
        vec![]
    }

    #[instrument(skip(self), fields(uri))]
    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        debug!(uri, "Reading resource");
        let resources = self.resources.clone();
        let uri = uri.to_string();

        Box::pin(async move {
            match resources.read(&uri).await {
                Ok(content) => {
                    info!(uri, "Resource read successfully");
                    Ok(content)
                }
                Err(e) => {
                    debug!(uri, error = %e, "Resource read failed");
                    Err(ResourceError::NotFound(e.to_string()))
                }
            }
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        // Prompts not yet implemented
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt {} not found - prompts not yet implemented",
                prompt_name
            )))
        })
    }
}

/// Builder for Botticelli MCP router.
#[derive(Default)]
pub struct BotticelliRouterBuilder {
    name: Option<String>,
    version: Option<String>,
    tools: Option<ToolRegistry>,
    resources: Option<ResourceRegistry>,
    metrics: Option<Arc<PrometheusMetrics>>,
}

impl BotticelliRouterBuilder {
    /// Sets the server name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the server version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the tool registry.
    pub fn tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Sets the resource registry.
    pub fn resources(mut self, resources: ResourceRegistry) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Sets the metrics collector.
    pub fn metrics(mut self, metrics: Arc<PrometheusMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Builds the router.
    pub fn build(self) -> BotticelliRouter {
        let metrics = self
            .metrics
            .unwrap_or_else(|| Arc::new(PrometheusMetrics::new()));

        let tools = if let Some(tools) = self.tools {
            tools
        } else {
            // Create default registry with metrics
            let mut registry = ToolRegistry::with_metrics(Arc::clone(&metrics));
            // Register default tools
            registry.register(Arc::new(crate::tools::EchoTool));
            registry.register(Arc::new(crate::tools::ServerInfoTool));
            registry.register(Arc::new(crate::tools::ValidateNarrativeTool));
            registry.register(Arc::new(crate::tools::GenerateTool));
            registry.register(Arc::new(crate::tools::ExecuteActTool::new()));
            registry.register(Arc::new(crate::tools::ExecuteNarrativeTool::new()));
            registry.register(Arc::new(crate::tools::ExportMetricsTool::new(Arc::clone(
                &metrics,
            ))));
            registry
        };

        BotticelliRouter {
            name: self.name.unwrap_or_else(|| "botticelli".to_string()),
            version: self.version.unwrap_or_else(|| "0.1.0".to_string()),
            tools,
            resources: self.resources.unwrap_or_default(),
            metrics,
        }
    }
}
