//! rmcp server implementation for Botticelli.

mod tool_impls;

use elicitation::{DynamicToolRegistry, ElicitPlugin as _};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler};
use std::sync::Arc;
use tracing::{debug, instrument};

/// Botticelli MCP server.
pub struct BotticelliServer {
    pub(crate) tool_router: ToolRouter<Self>,
    pub(crate) dynamic: DynamicToolRegistry,
    pub(crate) metrics: Arc<crate::PrometheusMetrics>,
    #[cfg(feature = "gemini")]
    pub(crate) gemini: Option<Arc<botticelli_models::GeminiClient>>,
    #[cfg(feature = "anthropic")]
    pub(crate) anthropic: Option<Arc<botticelli_models::AnthropicClient>>,
    #[cfg(feature = "ollama")]
    pub(crate) ollama: Option<Arc<botticelli_models::OllamaClient>>,
    #[cfg(feature = "huggingface")]
    pub(crate) huggingface: Option<Arc<botticelli_models::HuggingFaceDriver>>,
    #[cfg(feature = "groq")]
    pub(crate) groq: Option<Arc<botticelli_models::GroqDriver>>,
    #[cfg(feature = "discord")]
    pub(crate) discord_client: Option<Arc<crate::tools::discord::DiscordClient>>,
}

impl Default for BotticelliServer {
    #[instrument]
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for BotticelliServer {
    #[instrument(skip(self))]
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder().enable_tools().build();
        ServerInfo::new(capabilities)
            .with_instructions("Botticelli MCP server — narrative creation and execution.")
    }

    #[instrument(skip(self, _request, _context))]
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let mut tools = self.tool_router.list_all();
        tools.extend(self.dynamic.list_tools());
        debug!(count = tools.len(), "Listing tools");
        std::future::ready(Ok(ListToolsResult {
            tools,
            ..Default::default()
        }))
    }

    #[instrument(skip(self, context), fields(tool = %request.name))]
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            if self.tool_router.has_route(request.name.as_ref()) {
                let ctx = ToolCallContext::new(self, request, context);
                return self.tool_router.call(ctx).await;
            }
            self.dynamic.call_tool(request, context).await
        }
    }
}
