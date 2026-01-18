//! BotticelliServer struct and builder.

use crate::dialog_resource::DialogResource;
use crate::PrometheusMetrics;
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
///     .build();
/// ```
#[derive(Clone)]
pub struct BotticelliServer {
    pub(crate) tool_router: ToolRouter<Self>,

    #[cfg(feature = "database")]
    pub(super) db_ops: Option<
        Arc<dyn DatabaseRegistryOperations<Error = botticelli_error::BotticelliError>>,
    >,

    pub(super) dialog: Option<Arc<DialogResource>>,

    pub(super) metrics: Option<Arc<PrometheusMetrics>>,

    pub(super) narrative_registry: Arc<crate::tools::PartialNarrativeRegistry>,

    #[cfg(feature = "gemini")]
    pub(super) gemini_driver: Option<Arc<botticelli_models::GeminiClient>>,

    #[cfg(feature = "anthropic")]
    pub(super) anthropic_driver: Option<Arc<botticelli_models::AnthropicClient>>,

    #[cfg(feature = "ollama")]
    pub(super) ollama_driver: Option<Arc<botticelli_models::OllamaClient>>,

    #[cfg(feature = "huggingface")]
    pub(super) huggingface_driver: Option<Arc<botticelli_models::HuggingFaceDriver>>,

    #[cfg(feature = "groq")]
    pub(super) groq_driver: Option<Arc<botticelli_models::GroqDriver>>,
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
    ///     .build();
    /// ```
    #[tracing::instrument]
    pub fn builder() -> BotticelliServerBuilder {
        BotticelliServerBuilder::default()
    }
}

/// Builder for BotticelliServer.
///
/// Provides a type-safe way to configure optional server components
/// before construction.
#[derive(Clone, Default)]
pub struct BotticelliServerBuilder {
    #[cfg(feature = "database")]
    db_ops: Option<Arc<dyn DatabaseRegistryOperations<Error = botticelli_error::BotticelliError>>>,

    dialog: Option<Arc<DialogResource>>,

    metrics: Option<Arc<PrometheusMetrics>>,

    narrative_registry: Option<Arc<crate::tools::PartialNarrativeRegistry>>,

    #[cfg(feature = "gemini")]
    gemini_driver: Option<Arc<botticelli_models::GeminiClient>>,

    #[cfg(feature = "anthropic")]
    anthropic_driver: Option<Arc<botticelli_models::AnthropicClient>>,

    #[cfg(feature = "ollama")]
    ollama_driver: Option<Arc<botticelli_models::OllamaClient>>,

    #[cfg(feature = "huggingface")]
    huggingface_driver: Option<Arc<botticelli_models::HuggingFaceDriver>>,

    #[cfg(feature = "groq")]
    groq_driver: Option<Arc<botticelli_models::GroqDriver>>,
}

impl BotticelliServerBuilder {
    /// Configure database operations.
    ///
    /// # Arguments
    ///
    /// * `db` - Database operations implementation
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[cfg(feature = "database")]
    #[tracing::instrument(skip(self, db))]
    pub fn database(
        mut self,
        db: Arc<dyn DatabaseRegistryOperations<Error = botticelli_error::BotticelliError>>,
    ) -> Self {
        self.db_ops = Some(db);
        self
    }

    /// Configure dialog resource for elicitation tools.
    ///
    /// # Arguments
    ///
    /// * `dialog` - Dialog resource for user interaction
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[tracing::instrument(skip(self, dialog))]
    pub fn dialog(mut self, dialog: Arc<DialogResource>) -> Self {
        self.dialog = Some(dialog);
        self
    }

    /// Configure Prometheus metrics collector.
    ///
    /// # Arguments
    ///
    /// * `metrics` - Prometheus metrics collector
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[tracing::instrument(skip(self, metrics))]
    pub fn metrics(mut self, metrics: Arc<PrometheusMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Configure Gemini LLM driver.
    ///
    /// # Arguments
    ///
    /// * `driver` - Gemini client
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[cfg(feature = "gemini")]
    #[tracing::instrument(skip(self, driver))]
    pub fn gemini(mut self, driver: Arc<botticelli_models::GeminiClient>) -> Self {
        self.gemini_driver = Some(driver);
        self
    }

    /// Configure Anthropic LLM driver.
    ///
    /// # Arguments
    ///
    /// * `driver` - Anthropic client
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[cfg(feature = "anthropic")]
    #[tracing::instrument(skip(self, driver))]
    pub fn anthropic(mut self, driver: Arc<botticelli_models::AnthropicClient>) -> Self {
        self.anthropic_driver = Some(driver);
        self
    }

    /// Configure Ollama LLM driver.
    ///
    /// # Arguments
    ///
    /// * `driver` - Ollama client
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[cfg(feature = "ollama")]
    #[tracing::instrument(skip(self, driver))]
    pub fn ollama(mut self, driver: Arc<botticelli_models::OllamaClient>) -> Self {
        self.ollama_driver = Some(driver);
        self
    }

    /// Configure HuggingFace LLM driver.
    ///
    /// # Arguments
    ///
    /// * `driver` - HuggingFace driver
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[cfg(feature = "huggingface")]
    #[tracing::instrument(skip(self, driver))]
    pub fn huggingface(mut self, driver: Arc<botticelli_models::HuggingFaceDriver>) -> Self {
        self.huggingface_driver = Some(driver);
        self
    }

    /// Configure Groq LLM driver.
    ///
    /// # Arguments
    ///
    /// * `driver` - Groq driver
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[cfg(feature = "groq")]
    #[tracing::instrument(skip(self, driver))]
    pub fn groq(mut self, driver: Arc<botticelli_models::GroqDriver>) -> Self {
        self.groq_driver = Some(driver);
        self
    }

    /// Build the BotticelliServer instance.
    ///
    /// # Returns
    ///
    /// A configured `BotticelliServer` ready to serve MCP requests.
    #[tracing::instrument(skip(self))]
    pub fn build(self) -> BotticelliServer {
        BotticelliServer {
            tool_router: super::tools::get_tool_router(),
            #[cfg(feature = "database")]
            db_ops: self.db_ops,
            dialog: self.dialog,
            metrics: self.metrics,
            narrative_registry: self
                .narrative_registry
                .unwrap_or_else(|| Arc::new(crate::tools::PartialNarrativeRegistry::new())),
            #[cfg(feature = "gemini")]
            gemini_driver: self.gemini_driver,
            #[cfg(feature = "anthropic")]
            anthropic_driver: self.anthropic_driver,
            #[cfg(feature = "ollama")]
            ollama_driver: self.ollama_driver,
            #[cfg(feature = "huggingface")]
            huggingface_driver: self.huggingface_driver,
            #[cfg(feature = "groq")]
            groq_driver: self.groq_driver,
        }
    }
}
