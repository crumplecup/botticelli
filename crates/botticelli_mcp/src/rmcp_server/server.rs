//! BotticelliServer struct and builder.

use crate::dialog_resource::DialogResource;
use crate::PrometheusMetrics;
use derive_builder::Builder;
use derive_getters::Getters;
use rmcp::handler::server::tool::ToolRouter;
use std::sync::Arc;

#[cfg(feature = "database")]
use botticelli_interface::DatabaseRegistryOperations;

/// Botticelli MCP server using rmcp.
///
/// This server exposes Botticelli's capabilities as MCP tools.
/// Use the builder pattern to construct instances with optional components.
///
/// # Examples
///
/// ```no_run
/// use botticelli_mcp::BotticelliServer;
///
/// let server = BotticelliServer::builder()
///     .build()
///     .expect("Valid server");
/// ```
#[derive(Clone, Getters, Builder)]
#[builder(pattern = "owned")]
pub struct BotticelliServer {
    /// Tool router for handling MCP tool requests.
    #[getter(rename = "get_tool_router")]
    #[builder(setter(skip))]
    tool_router: ToolRouter<Self>,

    /// Database operations (optional).
    #[cfg(feature = "database")]
    #[builder(default)]
    db_ops: Option<Arc<dyn DatabaseRegistryOperations<Error = botticelli_error::BotticelliError>>>,

    /// Dialog resource for elicitation tools (optional).
    #[builder(default)]
    dialog: Option<Arc<DialogResource>>,

    /// Prometheus metrics collector (optional).
    #[builder(default)]
    metrics: Option<Arc<PrometheusMetrics>>,

    /// Partial narrative registry.
    #[builder(setter(into), default = "Arc::new(crate::tools::PartialNarrativeRegistry::new())")]
    narrative_registry: Arc<crate::tools::PartialNarrativeRegistry>,

    /// Gemini LLM driver (optional).
    #[cfg(feature = "gemini")]
    #[builder(default)]
    gemini_driver: Option<Arc<botticelli_models::GeminiClient>>,

    /// Anthropic LLM driver (optional).
    #[cfg(feature = "anthropic")]
    #[builder(default)]
    anthropic_driver: Option<Arc<botticelli_models::AnthropicClient>>,

    /// Ollama LLM driver (optional).
    #[cfg(feature = "ollama")]
    #[builder(default)]
    ollama_driver: Option<Arc<botticelli_models::OllamaClient>>,

    /// HuggingFace LLM driver (optional).
    #[cfg(feature = "huggingface")]
    #[builder(default)]
    huggingface_driver: Option<Arc<botticelli_models::HuggingFaceDriver>>,

    /// Groq LLM driver (optional).
    #[cfg(feature = "groq")]
    #[builder(default)]
    groq_driver: Option<Arc<botticelli_models::GroqDriver>>,
}

impl BotticelliServer {
    /// Create a builder for configuring the server.
    ///
    /// # Returns
    ///
    /// A new `BotticelliServerBuilder` with default configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use botticelli_mcp::BotticelliServer;
    ///
    /// let server = BotticelliServer::builder()
    ///     .build()
    ///     .expect("Valid server");
    /// ```
    #[tracing::instrument]
    pub fn builder() -> BotticelliServerBuilder {
        BotticelliServerBuilder::default()
    }
}
