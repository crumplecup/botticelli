//! RMCP-based MCP server implementation.
//!
//! The BotticelliServer struct holds all tool implementations and state
//! needed for MCP operations.

use crate::dialog_resource::DialogResource;
use crate::tools::narrative_validation_helpers::{
    add_helpful_comments, auto_fix_common_issues, format_toml, format_validation_result,
};
use crate::tools::NarrativeHelper;
use crate::{
    ApplyValidationFixesParams, ApplyValidationFixesResult, CarouselLevel, CarouselSummary,
    CreateNarrativeParams, CreateNarrativeResult, CreateNarrativeSessionParams,
    CreateNarrativeSessionResult, CreateSceneParams, CreateSceneResult, DeleteSceneParams,
    DeleteSceneResult, DiscordAuthor, DiscordChannelInfo, DiscordGetChannelsParams,
    DiscordGetChannelsResult, DiscordGetGuildInfoParams, DiscordGetGuildInfoResult,
    DiscordGetMessagesParams, DiscordGetMessagesResult, DiscordMessageInfo,
    DiscordPostMessageParams, DiscordPostMessageResult, EchoParams, EchoResult, ElicitActParams,
    ElicitActResult, ElicitBoolParams, ElicitBoolResult, ElicitCarouselParams,
    ElicitCarouselResult, ElicitMetadataParams, ElicitMetadataResult, ElicitNumberParams,
    ElicitNumberResult, ElicitSelectParams, ElicitSelectResult, ElicitTextParams,
    ElicitTextResult, ExecuteActParams, ExecuteActResult, ExecuteNarrativeParams,
    ExecuteNarrativeResult, ExportMetricsParams, ExportMetricsResult, FinalizeNarrativeParams,
    FinalizeNarrativeResult, GenerateParams, GenerateResult, GetNarrativeStateParams,
    GetNarrativeStateResult, ListScenesParams, ListScenesResult, MetricsFormat,
    ModifyNarrativeParams, ModifyNarrativeResult, NarrativeAnalysis, NarrativeStateSummary,
    PrometheusMetrics, QueryContentParams, QueryContentResult, SaveNarrativeParams,
    SaveNarrativeResult, ServerInfoResult, StateFormat, UpdateSceneParams, UpdateSceneResult,
    ValidateNarrativeParams, ValidateNarrativeResult, ValidateNarrativeSessionParams,
    ValidateNarrativeSessionResult, ValidationError, ValidationIssue, ValidationLocation,
    ValidationSeverity, ValidationWarning,
};
use botticelli_narrative::validator::validate_narrative_toml;
use std::path::Path;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::ServerCapabilities;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use std::sync::Arc;
use tracing::{debug, instrument};

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
    tool_router: ToolRouter<Self>,

    #[cfg(feature = "database")]
    db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,

    dialog: Option<Arc<DialogResource>>,

    metrics: Option<Arc<PrometheusMetrics>>,

    narrative_registry: Arc<crate::tools::PartialNarrativeRegistry>,

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
    db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,

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
    pub fn database(mut self, db: Arc<dyn DatabaseRegistryOperations>) -> Self {
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
    pub fn groq(mut self, driver: Arc<botticelli_models::GroqDriver>) -> Self {
        self.groq_driver = Some(driver);
        self
    }

    /// Build the BotticelliServer instance.
    ///
    /// # Returns
    ///
    /// A configured `BotticelliServer` ready to serve MCP requests.
    pub fn build(self) -> BotticelliServer {
        BotticelliServer {
            tool_router: BotticelliServer::tool_router(),
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

impl BotticelliServer {
    /// Select the appropriate LLM driver based on model prefix.
    ///
    /// # Arguments
    ///
    /// * `model` - Model identifier (e.g., "gemini-2.0-flash", "claude-3-5-sonnet")
    ///
    /// # Returns
    ///
    /// Arc to trait object implementing BotticelliDriver.
    ///
    /// # Errors
    ///
    /// Returns error if no matching driver is configured or available.
    #[cfg(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    ))]
    fn select_driver(&self, model: &str) -> Result<Arc<dyn botticelli_interface::BotticelliDriver>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        #[cfg(feature = "gemini")]
        if model.starts_with("gemini") || model.starts_with("models/gemini") {
            return self
                .gemini_driver
                .as_ref()
                .cloned()
                .map(|driver| driver as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Gemini driver not configured"),
                        None,
                    )
                });
        }

        #[cfg(feature = "anthropic")]
        if model.starts_with("claude") {
            return self
                .anthropic_driver
                .as_ref()
                .cloned()
                .map(|driver| driver as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Anthropic driver not configured"),
                        None,
                    )
                });
        }

        #[cfg(feature = "ollama")]
        if model.starts_with("llama") || model.starts_with("mistral") || model.starts_with("codellama") {
            return self
                .ollama_driver
                .as_ref()
                .cloned()
                .map(|driver| driver as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Ollama driver not configured"),
                        None,
                    )
                });
        }

        #[cfg(feature = "huggingface")]
        if model.contains("huggingface") {
            return self
                .huggingface_driver
                .as_ref()
                .cloned()
                .map(|driver| driver as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("HuggingFace driver not configured"),
                        None,
                    )
                });
        }

        #[cfg(feature = "groq")]
        if model.contains("groq") {
            return self
                .groq_driver
                .as_ref()
                .cloned()
                .map(|driver| driver as Arc<dyn botticelli_interface::BotticelliDriver>)
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Groq driver not configured"),
                        None,
                    )
                });
        }

        Err(rmcp::ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            Cow::Owned(format!("No driver available for model: {}", model)),
            None,
        ))
    }
}


#[tool_router]
impl BotticelliServer {
    /// Echo back a message with timestamp.
    ///
    /// This tool is useful for testing MCP connectivity and verifying
    /// that the server is responding correctly.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the message to echo
    ///
    /// # Returns
    ///
    /// Returns the echoed message with a timestamp on success.
    ///
    /// # Errors
    ///
    /// This tool should not fail under normal circumstances.
    #[tool(description = "Echoes back the input message with a timestamp")]
    #[instrument(skip(self), fields(message))]
    pub async fn echo(
        &self,
        Parameters(EchoParams { message }): Parameters<EchoParams>,
    ) -> Result<Json<EchoResult>, rmcp::ErrorData> {
        debug!(?message, "Processing echo request");

        let result = EchoResult::new(message);

        debug!(result = ?result, "Echo completed successfully");
        Ok(Json(result))
    }

    /// Get server information and metadata.
    ///
    /// Returns server name, version, and available tool count.
    ///
    /// # Returns
    ///
    /// Server metadata including version and tool count.
    ///
    /// # Errors
    ///
    /// This tool should not fail under normal circumstances.
    #[tool(description = "Returns server metadata and version information")]
    #[instrument(skip(self))]
    pub async fn server_info(&self) -> Result<Json<ServerInfoResult>, rmcp::ErrorData> {
        debug!("Retrieving server information");

        let result = Json(ServerInfoResult::new(
            "botticelli".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            self.tool_router.list_all().len(),
        ));

        debug!(tool_count = result.0.tool_count, "Server info retrieved");
        Ok(result)
    }

    /// Query content from database tables.
    ///
    /// Returns a list of content items with their metadata from the specified table.
    /// Requires database feature and configuration.
    ///
    /// # Arguments
    ///
    /// * `params` - Query parameters including table name and limit
    ///
    /// # Returns
    ///
    /// Query results with status, table name, count, limit, and rows.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database feature is not enabled
    /// - Database operations are not configured
    /// - Query execution fails
    /// - Invalid table name or limit
    #[tool(description = "Query content from database tables")]
    #[instrument(skip(self), fields(table, limit))]
    pub async fn query_content(
        &self,
        Parameters(QueryContentParams { table, limit }): Parameters<QueryContentParams>,
    ) -> Result<Json<QueryContentResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(?table, limit, "Processing query_content request");

        #[cfg(not(feature = "database"))]
        {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Database feature not enabled"),
                None,
            ));
        }

        #[cfg(feature = "database")]
        {
            // Check if database operations are available
            let db_ops = self.db_ops.as_ref().ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Borrowed("Database operations not configured"),
                    None,
                )
            })?;

            // Clamp limit to valid range
            let limit = limit.clamp(1, 100);

            debug!(table = %table, limit, "Querying content");

            // Execute query
            let query = format!("SELECT * FROM {} LIMIT {}", table, limit);
            let rows = db_ops.execute_query(&query).await.map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Query failed: {}", e)),
                    None,
                )
            })?;

            debug!(count = rows.len(), "Retrieved rows from database");

            let result = QueryContentResult::new(table, limit, rows);
            Ok(Json(result))
        }
    }

    /// Export execution metrics in Prometheus or summary format.
    ///
    /// Returns metrics for monitoring dashboards. Can export in full Prometheus
    /// text format or as a quick summary.
    ///
    /// # Arguments
    ///
    /// * `params` - Export parameters including format selection
    ///
    /// # Returns
    ///
    /// Metrics in the requested format (prometheus or summary).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Metrics collector is not configured
    /// - Metrics export fails
    #[tool(description = "Export execution metrics in Prometheus text format for monitoring dashboards")]
    #[instrument(skip(self))]
    pub async fn export_metrics(
        &self,
        Parameters(ExportMetricsParams { format }): Parameters<ExportMetricsParams>,
    ) -> Result<Json<ExportMetricsResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(?format, "Exporting metrics");

        // Check if metrics collector is available
        let metrics = self.metrics.as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Metrics collector not configured"),
                None,
            )
        })?;

        match format {
            MetricsFormat::Prometheus => {
                let metrics_text = metrics.export_prometheus().map_err(|e| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Owned(format!("Failed to export prometheus metrics: {}", e)),
                        None,
                    )
                })?;

                let result = ExportMetricsResult::prometheus(metrics_text);
                Ok(Json(result))
            }
            MetricsFormat::Summary => {
                let summary = metrics.summary().map_err(|e| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Owned(format!("Failed to get metrics summary: {}", e)),
                        None,
                    )
                })?;

                let result = ExportMetricsResult::summary(
                    summary.total_executions,
                    summary.total_tokens,
                    summary.total_cost_usd,
                    summary.avg_duration_ms,
                );
                Ok(Json(result))
            }
        }
    }

    /// Elicit free-form text input from the user.
    ///
    /// This tool provides the basic building block for text elicitation
    /// that the elicitation crate's derive macros expect.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the prompt to display
    ///
    /// # Returns
    ///
    /// The user's text input.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Dialog resource is not configured
    /// - Dialog interaction fails
    #[tool(description = "Elicit free-form text input from the user")]
    #[instrument(skip(self))]
    pub async fn elicit_text(
        &self,
        Parameters(ElicitTextParams { prompt }): Parameters<ElicitTextParams>,
    ) -> Result<Json<ElicitTextResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(?prompt, "Eliciting text input");

        // Check if dialog resource is available
        let dialog = self.dialog.as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Dialog resource not configured"),
                None,
            )
        })?;

        // Ask for text input
        let text = dialog.ask_text(&prompt).await.map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Dialog error: {}", e)),
                None,
            )
        })?;

        debug!(response_len = text.len(), "Received text input");

        let result = ElicitTextResult::new(text);
        Ok(Json(result))
    }

    /// Elicit a yes/no confirmation from the user.
    ///
    /// This tool provides the basic building block for boolean elicitation
    /// that the elicitation crate's derive macros expect.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the prompt and optional default value
    ///
    /// # Returns
    ///
    /// The user's boolean confirmation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Dialog resource is not configured
    /// - Dialog interaction fails
    #[tool(description = "Elicit a yes/no confirmation from the user")]
    #[instrument(skip(self))]
    pub async fn elicit_bool(
        &self,
        Parameters(ElicitBoolParams { prompt, default }): Parameters<ElicitBoolParams>,
    ) -> Result<Json<ElicitBoolResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(?prompt, default, "Eliciting boolean confirmation");

        // Check if dialog resource is available
        let dialog = self.dialog.as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Dialog resource not configured"),
                None,
            )
        })?;

        // Ask for confirmation
        let confirmed = dialog
            .ask_confirmation(&prompt, default)
            .await
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Dialog error: {}", e)),
                    None,
                )
            })?;

        debug!(confirmed, "Received boolean confirmation");

        let result = ElicitBoolResult::new(confirmed);
        Ok(Json(result))
    }

    /// Elicit a number within a specified range.
    ///
    /// This tool provides the basic building block for numeric elicitation
    /// that the elicitation crate's derive macros expect.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the prompt, min, and max values
    ///
    /// # Returns
    ///
    /// The user's numeric input within the specified range.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Dialog resource is not configured
    /// - Dialog interaction fails
    /// - Invalid range (min > max)
    #[tool(description = "Elicit a number within a specified range (min and max inclusive)")]
    #[instrument(skip(self))]
    pub async fn elicit_number(
        &self,
        Parameters(ElicitNumberParams { prompt, min, max }): Parameters<ElicitNumberParams>,
    ) -> Result<Json<ElicitNumberResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(?prompt, min, max, "Eliciting numeric input");

        // Validate range
        if min > max {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Invalid range: min ({}) > max ({})", min, max)),
                None,
            ));
        }

        // Check if dialog resource is available
        let dialog = self.dialog.as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Dialog resource not configured"),
                None,
            )
        })?;

        // Ask for number input
        let number = dialog.ask_number(&prompt, min, max).await.map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Dialog error: {}", e)),
                None,
            )
        })?;

        debug!(number, "Received numeric input");

        let result = ElicitNumberResult::new(number);
        Ok(Json(result))
    }

    /// Select one option from a finite list of choices.
    ///
    /// This tool provides the basic building block for selection elicitation
    /// that the elicitation crate's derive macros expect.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the prompt and options list
    ///
    /// # Returns
    ///
    /// The user's selected option from the provided list.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Dialog resource is not configured
    /// - Dialog interaction fails
    /// - Options array is empty
    /// - Selected index is out of bounds
    #[tool(description = "Select one option from a finite list of choices")]
    #[instrument(skip(self))]
    pub async fn elicit_select(
        &self,
        Parameters(ElicitSelectParams { prompt, options }): Parameters<ElicitSelectParams>,
    ) -> Result<Json<ElicitSelectResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(?prompt, option_count = options.len(), "Eliciting selection");

        // Validate options is not empty
        if options.is_empty() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("Options array cannot be empty"),
                None,
            ));
        }

        // Check if dialog resource is available
        let dialog = self.dialog.as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Dialog resource not configured"),
                None,
            )
        })?;

        // Convert to &str array for dialog API
        let option_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();

        // Ask for selection
        let index = dialog
            .ask_choice(&prompt, &option_refs)
            .await
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Dialog error: {}", e)),
                    None,
                )
            })?;

        // Get selected option
        let selected = options.get(index).ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Invalid index {} (max {})", index, options.len())),
                None,
            )
        })?;

        debug!(selected = %selected, index, "Received selection");

        let result = ElicitSelectResult::new(selected.clone());
        Ok(Json(result))
    }

    /// Create a new scene in a narrative.
    ///
    /// This tool creates a new scene with the given name and optional description.
    /// Scene management is currently a placeholder for future narrative editing features.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing narrative ID, scene name, and optional description
    ///
    /// # Returns
    ///
    /// The created scene with generated ID and metadata.
    #[tool(description = "Create a new scene in a narrative")]
    #[instrument(skip(self))]
    pub async fn create_scene(
        &self,
        Parameters(CreateSceneParams {
            narrative_id,
            scene_name,
            description,
        }): Parameters<CreateSceneParams>,
    ) -> Result<Json<CreateSceneResult>, rmcp::ErrorData> {
        debug!(
            narrative_id = %narrative_id,
            scene_name = %scene_name,
            has_description = description.is_some(),
            "Creating scene in narrative"
        );

        // Generate a new scene ID
        let scene_id = format!("scene_{}", uuid::Uuid::new_v4());

        debug!(scene_id = %scene_id, "Generated scene ID");

        let result = CreateSceneResult::new(scene_id, narrative_id, scene_name, description);
        Ok(Json(result))
    }

    /// List all scenes in a narrative.
    ///
    /// This tool lists scenes in a narrative. Currently returns an empty list
    /// as a placeholder for future implementation.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the narrative ID
    ///
    /// # Returns
    ///
    /// List of scenes in the narrative.
    #[tool(description = "List all scenes in a narrative")]
    #[instrument(skip(self))]
    pub async fn list_scenes(
        &self,
        Parameters(ListScenesParams { narrative_id }): Parameters<ListScenesParams>,
    ) -> Result<Json<ListScenesResult>, rmcp::ErrorData> {
        debug!(narrative_id = %narrative_id, "Listing scenes");

        let result = ListScenesResult::new(narrative_id, vec![]);
        Ok(Json(result))
    }

    /// Update scene details.
    ///
    /// This tool updates a scene's properties with the provided updates object.
    /// Currently a placeholder for future scene editing features.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing scene ID and updates object
    ///
    /// # Returns
    ///
    /// Confirmation of the scene update.
    #[tool(description = "Update scene details")]
    #[instrument(skip(self))]
    pub async fn update_scene(
        &self,
        Parameters(UpdateSceneParams { scene_id, updates }): Parameters<UpdateSceneParams>,
    ) -> Result<Json<UpdateSceneResult>, rmcp::ErrorData> {
        debug!(scene_id = %scene_id, "Updating scene");

        let result = UpdateSceneResult::new(scene_id, updates);
        Ok(Json(result))
    }

    /// Delete a scene from a narrative.
    ///
    /// This tool deletes a scene by ID. Currently a placeholder for future
    /// scene management features.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the scene ID to delete
    ///
    /// # Returns
    ///
    /// Confirmation of the scene deletion.
    #[tool(description = "Delete a scene from a narrative")]
    #[instrument(skip(self))]
    pub async fn delete_scene(
        &self,
        Parameters(DeleteSceneParams { scene_id }): Parameters<DeleteSceneParams>,
    ) -> Result<Json<DeleteSceneResult>, rmcp::ErrorData> {
        debug!(scene_id = %scene_id, "Deleting scene");

        let result = DeleteSceneResult::new(scene_id);
        Ok(Json(result))
    }

    /// Generate a complete narrative TOML from a natural language description.
    ///
    /// This tool creates a narrative workflow by analyzing a description and
    /// generating the necessary TOML structure including metadata, table of
    /// contents, and act definitions. The generated narrative is validated
    /// and auto-fixed for common issues.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing description, name, and optional defaults
    ///
    /// # Returns
    ///
    /// Complete narrative TOML with validation results and summary.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Name is invalid (must be alphanumeric with underscores)
    /// - Description is empty
    /// - TOML generation fails
    #[tool(
        description = "Generate a complete narrative TOML from a natural language description"
    )]
    #[instrument(skip(self))]
    pub async fn create_narrative(
        &self,
        Parameters(CreateNarrativeParams {
            description,
            name,
            default_model,
            default_temperature,
        }): Parameters<CreateNarrativeParams>,
    ) -> Result<Json<CreateNarrativeResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(name = %name, has_model = default_model.is_some(), "Creating narrative from description");

        // Validate name
        if !NarrativeHelper::is_valid_name(&name) {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!(
                    "Invalid narrative name '{}'. Must be alphanumeric with underscores, starting with a letter",
                    name
                )),
                None,
            ));
        }

        // Generate narrative TOML
        let mut toml = generate_narrative_toml(
            &description,
            &name,
            default_model.as_deref(),
            default_temperature,
        )?;

        // Auto-fix common issues
        let (fixed_toml, fixes_applied) = auto_fix_common_issues(&toml);
        toml = fixed_toml;

        // Format TOML
        toml = format_toml(&toml);

        // Add helpful comments
        let toml_with_comments = add_helpful_comments(&toml);

        // Validate
        let validation = validate_narrative_toml(&toml);

        debug!(
            valid = validation.is_valid(),
            errors = validation.errors.len(),
            warnings = validation.warnings.len(),
            fixes_applied = fixes_applied.len(),
            "Narrative generated and validated"
        );

        // Format validation results
        let validation_json = format_validation_result(&validation);

        // Generate summary
        let act_count = NarrativeHelper::count_acts(&toml);
        let summary = if validation.is_valid() {
            if fixes_applied.is_empty() {
                format!("Created narrative '{}' with {} act(s)", name, act_count)
            } else {
                format!(
                    "Created narrative '{}' with {} act(s) ({} auto-fixes applied)",
                    name,
                    act_count,
                    fixes_applied.len()
                )
            }
        } else {
            format!(
                "Generated narrative has {} error(s) - see validation for details",
                validation.errors.len()
            )
        };

        let result = CreateNarrativeResult::new(
            toml,
            toml_with_comments,
            validation_json,
            summary,
            fixes_applied,
            act_count,
        );

        Ok(Json(result))
    }

    /// Modify an existing narrative based on natural language instructions.
    ///
    /// This tool updates a narrative TOML by applying modifications described
    /// in natural language. Supports adding/removing acts, changing models,
    /// adjusting temperature, and adding bot commands. The modified narrative
    /// is validated and auto-fixed for common issues.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing existing TOML, modification instructions,
    ///   and optional save path
    ///
    /// # Returns
    ///
    /// Modified narrative TOML with validation results and change log.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Modification instruction is not understood
    /// - TOML structure is invalid
    /// - File save fails (if save_to is provided)
    #[tool(description = "Modify an existing narrative based on natural language instructions")]
    #[instrument(skip(self))]
    pub async fn modify_narrative(
        &self,
        Parameters(ModifyNarrativeParams {
            narrative_toml,
            modification,
            save_to,
        }): Parameters<ModifyNarrativeParams>,
    ) -> Result<Json<ModifyNarrativeResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(modification = %modification, has_save_path = save_to.is_some(), "Modifying narrative");

        // Apply modification
        let (mut modified_toml, mut changes) =
            apply_modification(&narrative_toml, &modification)?;

        // Auto-fix common issues
        let (fixed_toml, fixes_applied) = auto_fix_common_issues(&modified_toml);
        if !fixes_applied.is_empty() {
            modified_toml = fixed_toml;
            changes.extend(fixes_applied.iter().map(|f| format!("Auto-fix: {}", f)));
        }

        // Format TOML
        modified_toml = format_toml(&modified_toml);

        // Validate
        let validation = validate_narrative_toml(&modified_toml);

        debug!(
            valid = validation.is_valid(),
            changes = changes.len(),
            auto_fixes = fixes_applied.len(),
            "Narrative modified and validated"
        );

        // Optionally save to file
        let mut saved_to = None;
        if let Some(path) = save_to {
            tokio::fs::write(&path, &modified_toml)
                .await
                .map_err(|e| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Owned(format!("Failed to save file: {}", e)),
                        None,
                    )
                })?;
            saved_to = Some(path);
            debug!(path = saved_to.as_ref().unwrap(), "Saved modified narrative to file");
        }

        // Format validation results
        let validation_json = format_validation_result(&validation);

        let result = ModifyNarrativeResult::new(modified_toml, validation_json, changes, saved_to);
        Ok(Json(result))
    }

    /// Save a narrative TOML to a file.
    ///
    /// This tool persists a narrative to disk with path validation and
    /// overwrite protection. Creates parent directories as needed and
    /// returns the absolute path.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing TOML content, file path, and overwrite flag
    ///
    /// # Returns
    ///
    /// Confirmation with absolute path, file size, and overwrite status.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Path does not end with .toml extension
    /// - File exists and overwrite is false
    /// - Directory creation fails
    /// - File write fails
    #[tool(description = "Save a narrative TOML to a file")]
    #[instrument(skip(self))]
    pub async fn save_narrative(
        &self,
        Parameters(SaveNarrativeParams {
            narrative_toml,
            file_path,
            overwrite,
        }): Parameters<SaveNarrativeParams>,
    ) -> Result<Json<SaveNarrativeResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(path = %file_path, overwrite, "Saving narrative to file");

        // Validate path
        let path = Path::new(&file_path);

        // Check extension
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("File path must end with .toml extension"),
                None,
            ));
        }

        // Check if file exists
        let existed = path.exists();
        if existed && !overwrite {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!(
                    "File '{}' already exists. Set overwrite=true to replace it",
                    file_path
                )),
                None,
            ));
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Owned(format!("Failed to create directories: {}", e)),
                        None,
                    )
                })?;
                debug!(path = ?parent, "Created parent directories");
            }
        }

        // Write file
        tokio::fs::write(path, &narrative_toml)
            .await
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Failed to write file: {}", e)),
                    None,
                )
            })?;

        debug!(path = %file_path, "Narrative saved to file");

        // Get absolute path for response
        let absolute_path = std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.clone());

        let result = SaveNarrativeResult::new(absolute_path, narrative_toml.len(), existed);
        Ok(Json(result))
    }
    
    /// Validate a narrative TOML file or string.
    ///
    /// Checks syntax, structure, references, model names, and circular dependencies.
    /// Returns detailed errors and suggestions for fixing issues.
    ///
    /// # Arguments
    ///
    /// * `params` - Validation parameters including content or file path
    ///
    /// # Returns
    ///
    /// Validation result with errors, warnings, and suggestions.
    ///
    /// # Errors
    ///
    /// Returns error if neither content nor file_path is provided,
    /// or if file cannot be read.
    #[tool(description = "Validate a narrative TOML file or string with detailed error messages")]
    #[instrument(skip(self))]
    pub async fn validate_narrative(
        &self,
        Parameters(ValidateNarrativeParams {
            content,
            file_path,
            validate_files,
            validate_models,
            warn_unused,
            strict,
        }): Parameters<ValidateNarrativeParams>,
    ) -> Result<Json<ValidateNarrativeResult>, rmcp::ErrorData> {
        use botticelli_narrative::validator::{ValidationConfig, validate_narrative_toml_with_config};
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;
        use std::path::PathBuf;

        debug!(?file_path, has_content = content.is_some(), "Validating narrative");

        // Get TOML content
        let toml_content = if let Some(c) = content {
            c
        } else if let Some(ref path) = file_path {
            tokio::fs::read_to_string(path).await.map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    Cow::Owned(format!("Failed to read file '{}': {}", path, e)),
                    None,
                )
            })?
        } else {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("Either 'content' or 'file_path' must be provided"),
                None,
            ));
        };

        // Configure validation
        let config = ValidationConfig {
            validate_nested_narratives: validate_files,
            validate_media_files: validate_files,
            warn_unknown_models: validate_models,
            warn_unused_resources: warn_unused,
            base_dir: file_path
                .and_then(|p| PathBuf::from(p).parent().map(|parent| parent.to_path_buf())),
        };

        // Validate
        let result = validate_narrative_toml_with_config(&toml_content, &config);

        // Convert errors
        let errors: Vec<ValidationError> = result
            .errors
            .iter()
            .map(|e| ValidationError {
                kind: format!("{:?}", e.kind),
                message: e.message.clone(),
                suggestion: e.suggestion.clone(),
                location: e.location.as_ref().map(|loc| ValidationLocation {
                    line: loc.line,
                    column: loc.column,
                    section: loc.section.clone(),
                }),
            })
            .collect();

        // Convert warnings
        let warnings: Vec<ValidationWarning> = result
            .warnings
            .iter()
            .map(|w| ValidationWarning {
                kind: format!("{:?}", w.kind),
                message: w.message.clone(),
                location: w.location.as_ref().map(|loc| ValidationLocation {
                    line: loc.line,
                    column: loc.column,
                    section: loc.section.clone(),
                }),
            })
            .collect();

        let is_valid = result.is_valid();
        let has_warnings = !result.warnings.is_empty();
        let valid = is_valid && (!strict || !has_warnings);

        debug!(
            valid,
            errors = errors.len(),
            warnings = warnings.len(),
            "Validation complete"
        );

        Ok(Json(ValidateNarrativeResult::new(valid, errors, warnings)))
    }
    
    /// Generate text using an LLM.
    ///
    /// Simple text generation with configurable model, temperature, and tokens.
    /// Currently returns a placeholder response - full implementation requires
    /// LLM backend integration.
    ///
    /// # Arguments
    ///
    /// * `params` - Generation parameters including prompt and model
    ///
    /// # Returns
    ///
    /// Generated text response from the LLM.
    ///
    /// # Errors
    ///
    /// Returns error if LLM backend is not available or generation fails.
    #[tool(description = "Generate text using an LLM with configurable parameters")]
    #[instrument(skip(self))]
    pub async fn generate(
        &self,
        Parameters(GenerateParams {
            prompt,
            model,
            max_tokens,
            temperature,
            system_prompt,
        }): Parameters<GenerateParams>,
    ) -> Result<Json<GenerateResult>, rmcp::ErrorData> {
        debug!(%model, max_tokens, temperature, "Generating text");

        #[cfg(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        ))]
        {
            use botticelli_core::{GenerateRequest, Input, MessageBuilder, Role};
            use botticelli_interface::BotticelliDriver;

            // Select driver based on model
            let driver = self.select_driver(&model)?;

            // Build request
            let mut messages = Vec::new();

            if let Some(sys_prompt) = system_prompt {
                messages.push(
                    MessageBuilder::default()
                        .role(Role::System)
                        .content(vec![Input::Text(sys_prompt)])
                        .build()
                        .map_err(|e| {
                            rmcp::ErrorData::new(
                                ErrorCode::INTERNAL_ERROR,
                                Cow::Owned(format!("Failed to build system message: {}", e)),
                                None,
                            )
                        })?,
                );
            }

            messages.push(
                MessageBuilder::default()
                    .role(Role::User)
                    .content(vec![Input::Text(prompt.clone())])
                    .build()
                    .map_err(|e| {
                        rmcp::ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            Cow::Owned(format!("Failed to build user message: {}", e)),
                            None,
                        )
                    })?,
            );

            let request = GenerateRequest::builder()
                .messages(messages)
                .max_tokens(Some(max_tokens as usize))
                .temperature(Some(temperature as f64))
                .build()
                .map_err(|e| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Owned(format!("Failed to build request: {}", e)),
                        None,
                    )
                })?;

            // Execute generation
            let response = driver.generate(&request).await.map_err(|e| {
                tracing::error!(error = ?e, "LLM generation failed");
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Generation failed: {}", e)),
                    None,
                )
            })?;

            // Extract text from response
            let text = response
                .outputs()
                .first()
                .map(|output| match output {
                    botticelli_core::Output::Text(t) => t.clone(),
                    _ => format!("{:?}", output),
                })
                .unwrap_or_else(|| "No text generated".to_string());

            let tokens_used = response.usage().map(|u| *u.total_tokens() as u32);

            debug!(response_len = text.len(), ?tokens_used, "Generated text");

            Ok(Json(GenerateResult::new(text, model, tokens_used)))
        }

        #[cfg(not(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        )))]
        {
            // Placeholder implementation for testing when no LLM backends enabled
            let response_text = format!(
                "Placeholder response for: {}\n\nModel: {}\nMax tokens: {}\nTemperature: {}",
                prompt, model, max_tokens, temperature
            );

            debug!(response_len = response_text.len(), "Generated placeholder text");

            Ok(Json(GenerateResult::new(
                response_text,
                model,
                Some(max_tokens),
            )))
        }
    }
    
    /// Execute a single narrative act with an LLM.
    ///
    /// Executes one act/step of a narrative with context from previous acts.
    /// Currently returns a placeholder response - full implementation requires
    /// LLM backend integration.
    ///
    /// # Arguments
    ///
    /// * `params` - Act execution parameters including prompt and context
    ///
    /// # Returns
    ///
    /// Result from executing the act.
    ///
    /// # Errors
    ///
    /// Returns error if LLM backend is not available or execution fails.
    #[tool(description = "Execute a single narrative act with an LLM backend")]
    #[instrument(skip(self))]
    pub async fn execute_act(
        &self,
        Parameters(ExecuteActParams {
            prompt,
            model,
            max_tokens,
            temperature,
            system_prompt,
            context,
        }): Parameters<ExecuteActParams>,
    ) -> Result<Json<ExecuteActResult>, rmcp::ErrorData> {
        
        

        debug!(%model, has_context = context.is_some(), "Executing act");

        // Placeholder implementation
        // Full implementation would:
        // 1. Select driver based on model prefix
        // 2. Build message with system prompt, context, and user prompt
        // 3. Execute with driver
        // 4. Return response with token usage
        
        let response = format!(
            "Act execution placeholder\n\nPrompt: {}\nModel: {}\nContext: {}\n\nFull execution requires LLM backend integration.",
            prompt,
            model,
            context.as_deref().unwrap_or("(none)")
        );

        debug!(response_len = response.len(), "Act execution complete");

        Ok(Json(ExecuteActResult::new(
            response,
            model,
            Some(max_tokens),
            true,
        )))
    }
    
    /// Execute a complete narrative from a TOML file.
    ///
    /// Loads a narrative TOML, executes all acts in sequence with the given
    /// prompt, and returns the final output. Currently returns a placeholder
    /// response - full implementation requires LLM backend and narrative
    /// executor integration.
    ///
    /// # Arguments
    ///
    /// * `params` - Narrative execution parameters including file path and prompt
    ///
    /// # Returns
    ///
    /// Result from executing the narrative.
    ///
    /// # Errors
    ///
    /// Returns error if narrative file cannot be loaded, LLM backend is not
    /// available, or execution fails.
    #[tool(description = "Execute a complete narrative from a TOML file")]
    #[instrument(skip(self))]
    pub async fn execute_narrative(
        &self,
        Parameters(ExecuteNarrativeParams {
            narrative_path,
            prompt,
            model,
            max_tokens,
        }): Parameters<ExecuteNarrativeParams>,
    ) -> Result<Json<ExecuteNarrativeResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(%narrative_path, model = ?model, "Executing narrative");

        // Validate path exists
        let path = std::path::Path::new(&narrative_path);
        if !path.exists() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative file not found: {}", narrative_path)),
                None,
            ));
        }

        // Placeholder implementation
        // Full implementation would:
        // 1. Load and parse narrative TOML
        // 2. Create NarrativeExecutor with appropriate driver
        // 3. Execute all acts in sequence
        // 4. Track tokens and outputs
        // 5. Return final result
        
        let final_output = format!(
            "Narrative execution placeholder\n\nPath: {}\nPrompt: {}\nModel: {:?}\n\nFull execution requires LLM backend and narrative executor integration.",
            narrative_path,
            prompt,
            model
        );

        debug!("Narrative execution complete (placeholder)");

        Ok(Json(ExecuteNarrativeResult::new(
            final_output,
            0,  // acts_executed
            vec![model.unwrap_or_else(default_model)],
            Some(max_tokens),
            true,
            None,
        )))
    }

    // Session-based narrative elicitation tools

    /// Create a new narrative elicitation session.
    ///
    /// Analyzes the user's description to suggest a name and detect acts,
    /// then initializes a session for iterative narrative construction.
    ///
    /// # Arguments
    ///
    /// * `params` - Session creation parameters with user description
    ///
    /// # Returns
    ///
    /// Session ID, suggested name, and analysis of the description.
    #[tool(description = "Initialize a new narrative creation session from a description")]
    #[instrument(skip(self))]
    pub async fn create_narrative_session(
        &self,
        Parameters(CreateNarrativeSessionParams { description }): Parameters<
            CreateNarrativeSessionParams,
        >,
    ) -> Result<Json<CreateNarrativeSessionResult>, rmcp::ErrorData> {
        use crate::tools::NarrativeHelper;

        debug!(?description, "Creating narrative session");

        // Analyze description
        let acts = NarrativeHelper::extract_acts_from_description(&description);
        let suggested_name = NarrativeHelper::suggest_name_from_description(&description);

        let complexity = if acts.len() == 1 {
            "simple"
        } else if acts.len() <= 3 {
            "moderate"
        } else {
            "complex"
        };

        // Initialize session state
        let mut partial = crate::PartialNarrative::new();
        partial.description = Some(description.clone());
        partial.name = Some(suggested_name.clone());

        // Add acts
        for act in &acts {
            partial.acts.insert(
                act.name.clone(),
                crate::PartialAct::new(act.prompt.clone(), None, None, vec![], None),
            );
            partial.act_order.push(act.name.clone());
        }

        // Store in registry (returns the narrative name as the key/ID)
        let narrative_id = self.narrative_registry.add(partial);

        debug!(narrative_id = %narrative_id, acts = acts.len(), "Session created");

        Ok(Json(CreateNarrativeSessionResult {
            narrative_id,
            suggested_name,
            analysis: NarrativeAnalysis {
                detected_acts: acts.iter().map(|a| a.name.clone()).collect(),
                complexity: complexity.to_string(),
                act_count: acts.len(),
            },
        }))
    }

    /// Set or update narrative metadata.
    ///
    /// Updates name, description, and default model/temperature settings
    /// for a narrative elicitation session.
    ///
    /// # Arguments
    ///
    /// * `params` - Metadata parameters (all fields optional except narrative_id)
    ///
    /// # Returns
    ///
    /// Confirmation of update.
    #[tool(description = "Set or update narrative metadata (name, description, defaults)")]
    #[instrument(skip(self))]
    pub async fn elicit_metadata(
        &self,
        Parameters(ElicitMetadataParams {
            narrative_id,
            name,
            description,
            default_model,
            default_temperature,
        }): Parameters<ElicitMetadataParams>,
    ) -> Result<Json<ElicitMetadataResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(narrative_id, "Updating narrative metadata");

        // Update fields if provided
        self.narrative_registry
            .update(
                &narrative_id,
                serde_json::json!({
                    "name": name,
                    "description": description,
                    "model": default_model,
                    "temperature": default_temperature,
                }),
            )
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    Cow::Owned(format!("Failed to update metadata: {}", e)),
                    None,
                )
            })?;

        debug!(narrative_id, "Metadata updated");

        Ok(Json(ElicitMetadataResult {
            narrative_id,
            status: "updated".to_string(),
        }))
    }

    /// Add or update an act in the narrative.
    ///
    /// Creates a new act or updates an existing one with the given
    /// prompt and optional model/temperature overrides.
    ///
    /// # Arguments
    ///
    /// * `params` - Act parameters including name and prompt
    ///
    /// # Returns
    ///
    /// Confirmation with act name and status.
    #[tool(description = "Add or update an act in the narrative")]
    #[instrument(skip(self))]
    pub async fn elicit_act(
        &self,
        Parameters(ElicitActParams {
            narrative_id,
            act_name,
            prompt,
            model,
            temperature,
        }): Parameters<ElicitActParams>,
    ) -> Result<Json<ElicitActResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(narrative_id, act_name, "Eliciting act");

        // Get current narrative
        let mut partial = self.narrative_registry.get(&narrative_id).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative not found: {}", e)),
                None,
            )
        })?;

        // Check if act exists
        let status = if partial.acts.contains_key(&act_name) {
            "updated"
        } else {
            partial.act_order.push(act_name.clone());
            "created"
        };

        // Create or update act
        partial.acts.insert(
            act_name.clone(),
            crate::PartialAct::new(prompt, model, temperature, vec![], None),
        );

        // Update registry
        self.narrative_registry.add(partial);

        debug!(narrative_id, act_name, status, "Act elicited");

        Ok(Json(ElicitActResult {
            narrative_id,
            act_name,
            status: status.to_string(),
        }))
    }

    /// Finalize a narrative session and generate TOML.
    ///
    /// Completes the elicitation session, optionally validates the narrative,
    /// and generates the final TOML representation.
    ///
    /// # Arguments
    ///
    /// * `params` - Finalization parameters with session ID
    ///
    /// # Returns
    ///
    /// Success status, TOML output, and optional validation errors.
    #[tool(description = "Finalize a narrative session and generate TOML")]
    #[instrument(skip(self))]
    pub async fn finalize_narrative(
        &self,
        Parameters(FinalizeNarrativeParams {
            narrative_id,
            validate,
        }): Parameters<FinalizeNarrativeParams>,
    ) -> Result<Json<FinalizeNarrativeResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(narrative_id, validate, "Finalizing narrative");

        // Get narrative
        let partial = self.narrative_registry.get(&narrative_id).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative not found: {}", e)),
                None,
            )
        })?;

        // Convert to TOML
        let toml = toml::to_string_pretty(&partial).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Failed to serialize to TOML: {}", e)),
                None,
            )
        })?;

        // Validate if requested
        let validation_errors = if validate {
            use botticelli_narrative::validator::{ValidationConfig, validate_narrative_toml_with_config};

            let config = ValidationConfig::default();
            let result = validate_narrative_toml_with_config(&toml, &config);

            if !result.is_valid() {
                Some(
                    result
                        .errors
                        .iter()
                        .map(|e| e.message.clone())
                        .collect(),
                )
            } else {
                None
            }
        } else {
            None
        };

        let success = validation_errors.is_none();

        // Remove from registry (session complete)
        self.narrative_registry.remove(&narrative_id);

        debug!(narrative_id, success, "Narrative finalized");

        Ok(Json(FinalizeNarrativeResult {
            success,
            toml,
            validation_errors,
        }))
    }

    /// Get the current state of a narrative session.
    ///
    /// Returns information about session completeness, acts, and optionally
    /// the TOML representation.
    ///
    /// # Arguments
    ///
    /// * `params` - State query parameters with session ID and format
    ///
    /// # Returns
    ///
    /// Session state summary with optional TOML.
    #[tool(description = "Get current state and completeness of a narrative session")]
    #[instrument(skip(self))]
    pub async fn get_narrative_state(
        &self,
        Parameters(GetNarrativeStateParams {
            narrative_id,
            format,
        }): Parameters<GetNarrativeStateParams>,
    ) -> Result<Json<GetNarrativeStateResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(narrative_id, ?format, "Getting narrative state");

        // Get narrative
        let partial = self.narrative_registry.get(&narrative_id).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative not found: {}", e)),
                None,
            )
        })?;

        // Calculate state
        let acts_count = partial.acts.len();
        let acts: Vec<String> = partial.act_order.clone();
        let has_carousel =
            partial.carousel.is_some() || partial.acts.values().any(|act| act.carousel.is_some());

        let metadata_complete =
            partial.name.is_some() && partial.description.is_some() && partial.model.is_some();
        let acts_complete =
            !partial.acts.is_empty() && partial.acts.values().all(|act| !act.prompt.is_empty());
        let inputs_partial = partial.acts.values().any(|act| !act.inputs.is_empty());

        let mut completeness_score = 0;
        if metadata_complete {
            completeness_score += 33;
        }
        if acts_complete {
            completeness_score += 33;
        }
        if inputs_partial {
            completeness_score += 34;
        }

        // Generate TOML if requested
        let toml = if matches!(format, StateFormat::Toml) {
            Some(toml::to_string_pretty(&partial).map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Failed to convert to TOML: {}", e)),
                    None,
                )
            })?)
        } else {
            None
        };

        debug!(
            narrative_id,
            completeness = %completeness_score,
            acts_count,
            "Narrative state retrieved"
        );

        Ok(Json(GetNarrativeStateResult {
            narrative_id,
            state: NarrativeStateSummary {
                name: partial.name.clone(),
                acts_count,
                acts,
                completeness: format!("{}%", completeness_score),
                has_carousel,
            },
            toml,
        }))
    }

    /// Validate a narrative session.
    ///
    /// Checks the narrative for completeness, required fields, and structural
    /// correctness. Returns detailed errors and warnings.
    ///
    /// # Arguments
    ///
    /// * `params` - Validation parameters with session ID and strict flag
    ///
    /// # Returns
    ///
    /// Validation results with errors, warnings, and completeness.
    #[tool(description = "Validate a narrative session for completeness and correctness")]
    #[instrument(skip(self))]
    pub async fn validate_narrative_session(
        &self,
        Parameters(ValidateNarrativeSessionParams {
            narrative_id,
            strict,
        }): Parameters<ValidateNarrativeSessionParams>,
    ) -> Result<Json<ValidateNarrativeSessionResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(narrative_id, strict, "Validating narrative session");

        // Get narrative
        let partial = self.narrative_registry.get(&narrative_id).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative not found: {}", e)),
                None,
            )
        })?;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate name
        if partial.name.is_none() || partial.name.as_ref().is_some_and(|n| n.is_empty()) {
            errors.push(ValidationIssue {
                severity: ValidationSeverity::Critical,
                field: "name".to_string(),
                message: "Narrative name is required".to_string(),
                suggestion: "Provide a unique name for this narrative".to_string(),
                auto_fixable: false,
            });
        }

        // Validate description
        if partial.description.is_none()
            || partial.description.as_ref().is_some_and(|d| d.is_empty())
        {
            errors.push(ValidationIssue {
                severity: ValidationSeverity::High,
                field: "description".to_string(),
                message: "Narrative description is missing".to_string(),
                suggestion: "Add a description explaining what this narrative does".to_string(),
                auto_fixable: false,
            });
        }

        // Validate model
        if partial.model.is_none() {
            warnings.push(ValidationIssue {
                severity: ValidationSeverity::Medium,
                field: "model".to_string(),
                message: "Default model not specified".to_string(),
                suggestion: "Set a default model (e.g., 'gemini-2.0-flash-exp')".to_string(),
                auto_fixable: true,
            });
        }

        // Validate acts
        if partial.acts.is_empty() {
            errors.push(ValidationIssue {
                severity: ValidationSeverity::Critical,
                field: "acts".to_string(),
                message: "Narrative has no acts".to_string(),
                suggestion: "Add at least one act to the narrative".to_string(),
                auto_fixable: false,
            });
        } else {
            for (act_name, act) in &partial.acts {
                if act.prompt.is_empty() {
                    errors.push(ValidationIssue {
                        severity: ValidationSeverity::High,
                        field: format!("acts.{}.prompt", act_name),
                        message: format!("Act '{}' has empty prompt", act_name),
                        suggestion: "Provide a prompt for this act".to_string(),
                        auto_fixable: false,
                    });
                }

                if strict && act.model.is_none() && partial.model.is_none() {
                    warnings.push(ValidationIssue {
                        severity: ValidationSeverity::Low,
                        field: format!("acts.{}.model", act_name),
                        message: format!("Act '{}' has no model specified", act_name),
                        suggestion: "Set model for act or narrative default".to_string(),
                        auto_fixable: true,
                    });
                }
            }
        }

        // Calculate completeness
        let metadata_complete =
            partial.name.is_some() && partial.description.is_some() && partial.model.is_some();
        let acts_complete =
            !partial.acts.is_empty() && partial.acts.values().all(|act| !act.prompt.is_empty());

        let mut completeness_score = 0;
        if metadata_complete {
            completeness_score += 50;
        }
        if acts_complete {
            completeness_score += 50;
        }

        let auto_fixable_count = errors
            .iter()
            .chain(warnings.iter())
            .filter(|issue| issue.auto_fixable)
            .count();

        let is_valid = errors.is_empty();

        debug!(
            narrative_id,
            is_valid,
            errors = errors.len(),
            warnings = warnings.len(),
            "Validation complete"
        );

        Ok(Json(ValidateNarrativeSessionResult {
            narrative_id,
            is_valid,
            errors,
            warnings,
            completeness: format!("{}%", completeness_score),
            auto_fixable_count,
        }))
    }

    /// Apply automated fixes to validation issues.
    ///
    /// Automatically fixes common validation problems like missing defaults.
    /// Returns the list of fixes applied and remaining error count.
    ///
    /// # Arguments
    ///
    /// * `params` - Fix parameters with session ID and fix types
    ///
    /// # Returns
    ///
    /// Success status, list of applied fixes, and remaining error count.
    #[tool(description = "Apply automated fixes to resolve validation issues")]
    #[instrument(skip(self))]
    pub async fn apply_validation_fixes(
        &self,
        Parameters(ApplyValidationFixesParams {
            narrative_id,
            fix_types,
            confirm,
        }): Parameters<ApplyValidationFixesParams>,
    ) -> Result<Json<ApplyValidationFixesResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(narrative_id, ?fix_types, confirm, "Applying validation fixes");

        if !confirm {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("Must set confirm=true to apply fixes"),
                None,
            ));
        }

        // Get narrative
        let mut partial = self.narrative_registry.get(&narrative_id).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative not found: {}", e)),
                None,
            )
        })?;

        let mut fixes_applied = Vec::new();

        // Apply fixes based on type
        let fix_all = fix_types.contains(&"all".to_string());

        if fix_all || fix_types.contains(&"missing_defaults".to_string()) {
            if partial.model.is_none() {
                partial.model = Some("gemini-2.0-flash-exp".to_string());
                fixes_applied.push("Set default model to gemini-2.0-flash-exp".to_string());
            }

            if partial.temperature.is_none() {
                partial.temperature = Some(0.7);
                fixes_applied.push("Set default temperature to 0.7".to_string());
            }

            if partial.max_tokens.is_none() {
                partial.max_tokens = Some(1000);
                fixes_applied.push("Set default max_tokens to 1000".to_string());
            }
        }

        // Update narrative in registry
        self.narrative_registry.add(partial);

        // Count remaining errors by validating
        let remaining_errors = {
            let partial = self.narrative_registry.get(&narrative_id).map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Failed to retrieve updated narrative: {}", e)),
                    None,
                )
            })?;

            let mut errors = 0;

            if partial.name.is_none() || partial.name.as_ref().is_some_and(|n| n.is_empty()) {
                errors += 1;
            }

            if partial.description.is_none()
                || partial.description.as_ref().is_some_and(|d| d.is_empty())
            {
                errors += 1;
            }

            if partial.acts.is_empty() {
                errors += 1;
            } else {
                for act in partial.acts.values() {
                    if act.prompt.is_empty() {
                        errors += 1;
                    }
                }
            }

            errors
        };

        debug!(
            narrative_id,
            fixes_count = fixes_applied.len(),
            remaining_errors,
            "Validation fixes applied"
        );

        Ok(Json(ApplyValidationFixesResult {
            success: true,
            fixes_applied,
            remaining_errors,
        }))
    }

    /// Create carousel configuration for iterative refinement.
    ///
    /// Configures carousel settings for narrative or act-level iteration,
    /// enabling multi-pass refinement with budget tracking.
    ///
    /// # Arguments
    ///
    /// * `params` - Carousel parameters with level, iterations, and budget settings
    ///
    /// # Returns
    ///
    /// Success status and carousel configuration summary.
    #[tool(description = "Create carousel configuration for iterative narrative refinement")]
    #[instrument(skip(self))]
    pub async fn elicit_carousel(
        &self,
        Parameters(ElicitCarouselParams {
            narrative_id,
            level,
            act_name,
            iterations,
            continue_on_error,
            estimated_tokens_per_iteration,
            budget_multiplier,
        }): Parameters<ElicitCarouselParams>,
    ) -> Result<Json<ElicitCarouselResult>, rmcp::ErrorData> {
        use botticelli_narrative::CarouselConfig;
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(narrative_id, ?level, iterations, "Creating carousel configuration");

        // Get narrative
        let mut partial = self.narrative_registry.get(&narrative_id).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative not found: {}", e)),
                None,
            )
        })?;

        // Create carousel config
        let carousel_config = CarouselConfig::new(
            iterations,
            estimated_tokens_per_iteration.unwrap_or(1000) as u64,
        )
        .with_continue_on_error(continue_on_error);

        // Calculate budget warnings
        let mut budget_warnings = Vec::new();
        if let Some(tokens_per_iter) = estimated_tokens_per_iteration {
            let total_estimated = tokens_per_iter * iterations;
            let budget_threshold = (total_estimated as f64 * budget_multiplier) as u32;

            if total_estimated > 10_000 {
                budget_warnings.push(format!(
                    "High token estimate: {} tokens across {} iterations",
                    total_estimated, iterations
                ));
            }

            if budget_threshold > 50_000 {
                budget_warnings.push(format!(
                    "Budget threshold very high: {} tokens ({}x multiplier)",
                    budget_threshold, budget_multiplier
                ));
            }
        }

        // Apply carousel based on level
        match level {
            CarouselLevel::Narrative => {
                partial.carousel = Some(carousel_config);
            }
            CarouselLevel::Act => {
                let act_name_ref = act_name.as_ref().ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        Cow::Borrowed("act_name required for Act level carousel"),
                        None,
                    )
                })?;

                if let Some(act) = partial.acts.get_mut(act_name_ref) {
                    act.carousel = Some(carousel_config);
                } else {
                    return Err(rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        Cow::Owned(format!("Act '{}' not found", act_name_ref)),
                        None,
                    ));
                }
            }
        }

        // Update narrative in registry
        self.narrative_registry.add(partial);

        debug!(
            narrative_id,
            ?level,
            iterations,
            "Carousel configuration created"
        );

        Ok(Json(ElicitCarouselResult {
            success: true,
            carousel_config: CarouselSummary {
                level: match level {
                    CarouselLevel::Narrative => "narrative".to_string(),
                    CarouselLevel::Act => "act".to_string(),
                },
                act_name,
                iterations,
                estimated_total_tokens: estimated_tokens_per_iteration.map(|t| t * iterations),
                budget_warnings,
            },
        }))
    }

    // ========================================================================
    // Discord Integration Tools (feature-gated)
    // ========================================================================

    /// Post a message to a Discord channel.
    ///
    /// Sends a message to the specified Discord channel using the Discord API.
    /// Requires DISCORD_TOKEN environment variable to be set.
    ///
    /// # Arguments
    ///
    /// * `params` - Channel ID and message content
    ///
    /// # Returns
    ///
    /// Message ID, channel ID, and timestamp on success.
    ///
    /// # Errors
    ///
    /// Returns error if DISCORD_TOKEN is not set, channel doesn't exist,
    /// bot lacks permissions, or content exceeds 2000 characters.
    #[tool(description = "Post a message to a Discord channel")]
    #[instrument(skip(self))]
    pub async fn discord_post_message(
        &self,
        Parameters(DiscordPostMessageParams {
            channel_id,
            content,
        }): Parameters<DiscordPostMessageParams>,
    ) -> Result<Json<DiscordPostMessageResult>, rmcp::ErrorData> {
        use reqwest::Client;
        use rmcp::model::ErrorCode;
        use serde_json::json;
        use std::borrow::Cow;

        debug!(channel_id, content_len = content.len(), "Posting Discord message");

        // Validate content length
        if content.len() > 2000 {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("Content exceeds 2000 character limit"),
                None,
            ));
        }

        // Get Discord token
        let token = std::env::var("DISCORD_TOKEN").map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("DISCORD_TOKEN environment variable not set"),
                None,
            )
        })?;

        // Make Discord API request
        let client = Client::new();
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            channel_id
        );

        let response = client
            .post(&url)
            .header("Authorization", format!("Bot {}", token))
            .header("User-Agent", "Botticelli-MCP/0.2.0")
            .header("Content-Type", "application/json")
            .json(&json!({ "content": content }))
            .send()
            .await
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Discord API request failed: {}", e)),
                    None,
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Discord API error {}: {}", status, body)),
                None,
            ));
        }

        let message: serde_json::Value = response.json().await.map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Failed to parse Discord response: {}", e)),
                None,
            )
        })?;

        let message_id = message["id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let timestamp = message["timestamp"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        debug!(message_id, "Discord message posted successfully");

        Ok(Json(DiscordPostMessageResult {
            status: "success".to_string(),
            message_id,
            channel_id,
            timestamp,
        }))
    }

    /// Get message history from a Discord channel.
    ///
    /// Fetches recent messages from the specified Discord channel.
    /// Requires DISCORD_TOKEN environment variable to be set.
    ///
    /// # Arguments
    ///
    /// * `params` - Channel ID and optional limit (1-100, default 50)
    ///
    /// # Returns
    ///
    /// List of messages with content, timestamps, and author info.
    ///
    /// # Errors
    ///
    /// Returns error if DISCORD_TOKEN is not set, channel doesn't exist,
    /// or bot lacks permissions.
    #[tool(description = "Fetch message history from a Discord channel")]
    #[instrument(skip(self))]
    pub async fn discord_get_messages(
        &self,
        Parameters(DiscordGetMessagesParams {
            channel_id,
            limit,
        }): Parameters<DiscordGetMessagesParams>,
    ) -> Result<Json<DiscordGetMessagesResult>, rmcp::ErrorData> {
        use reqwest::Client;
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        let limit = limit.clamp(1, 100);
        debug!(channel_id, limit, "Getting Discord messages");

        // Get Discord token
        let token = std::env::var("DISCORD_TOKEN").map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("DISCORD_TOKEN environment variable not set"),
                None,
            )
        })?;

        // Make Discord API request
        let client = Client::new();
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages?limit={}",
            channel_id, limit
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bot {}", token))
            .header("User-Agent", "Botticelli-MCP/0.2.0")
            .send()
            .await
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Discord API request failed: {}", e)),
                    None,
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Discord API error {}: {}", status, body)),
                None,
            ));
        }

        let messages: Vec<serde_json::Value> = response.json().await.map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Failed to parse Discord response: {}", e)),
                None,
            )
        })?;

        let formatted_messages: Vec<DiscordMessageInfo> = messages
            .into_iter()
            .map(|m| DiscordMessageInfo {
                id: m["id"].as_str().unwrap_or("").to_string(),
                content: m["content"].as_str().unwrap_or("").to_string(),
                timestamp: m["timestamp"].as_str().unwrap_or("").to_string(),
                author: m["author"].as_object().map(|a| DiscordAuthor {
                    id: a["id"].as_str().unwrap_or("").to_string(),
                    username: a["username"].as_str().unwrap_or("").to_string(),
                }),
            })
            .collect();

        debug!(count = formatted_messages.len(), "Discord messages retrieved");

        Ok(Json(DiscordGetMessagesResult {
            status: "success".to_string(),
            channel_id,
            count: formatted_messages.len(),
            messages: formatted_messages,
        }))
    }

    /// Get information about a Discord guild (server).
    ///
    /// Fetches guild information including name and member count.
    /// Requires DISCORD_TOKEN environment variable to be set.
    ///
    /// # Arguments
    ///
    /// * `params` - Guild ID
    ///
    /// # Returns
    ///
    /// Guild ID, name, and member count.
    ///
    /// # Errors
    ///
    /// Returns error if DISCORD_TOKEN is not set, guild doesn't exist,
    /// or bot is not a member of the guild.
    #[tool(description = "Get information about a Discord guild (server)")]
    #[instrument(skip(self))]
    pub async fn discord_get_guild_info(
        &self,
        Parameters(DiscordGetGuildInfoParams { guild_id }): Parameters<
            DiscordGetGuildInfoParams,
        >,
    ) -> Result<Json<DiscordGetGuildInfoResult>, rmcp::ErrorData> {
        use reqwest::Client;
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(guild_id, "Getting Discord guild info");

        // Get Discord token
        let token = std::env::var("DISCORD_TOKEN").map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("DISCORD_TOKEN environment variable not set"),
                None,
            )
        })?;

        // Make Discord API request
        let client = Client::new();
        let url = format!(
            "https://discord.com/api/v10/guilds/{}?with_counts=true",
            guild_id
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bot {}", token))
            .header("User-Agent", "Botticelli-MCP/0.2.0")
            .send()
            .await
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Discord API request failed: {}", e)),
                    None,
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Discord API error {}: {}", status, body)),
                None,
            ));
        }

        let guild: serde_json::Value = response.json().await.map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Failed to parse Discord response: {}", e)),
                None,
            )
        })?;

        let name = guild["name"].as_str().unwrap_or("Unknown").to_string();
        let member_count = guild["approximate_member_count"].as_u64();

        debug!(guild_id, name, "Discord guild info retrieved");

        Ok(Json(DiscordGetGuildInfoResult {
            status: "success".to_string(),
            guild_id,
            name,
            member_count,
        }))
    }

    /// List channels in a Discord guild.
    ///
    /// Fetches all channels in the specified guild.
    /// Requires DISCORD_TOKEN environment variable to be set.
    ///
    /// # Arguments
    ///
    /// * `params` - Guild ID
    ///
    /// # Returns
    ///
    /// List of channels with IDs, names, and types.
    ///
    /// # Errors
    ///
    /// Returns error if DISCORD_TOKEN is not set, guild doesn't exist,
    /// or bot is not a member of the guild.
    #[tool(description = "List channels in a Discord guild")]
    #[instrument(skip(self))]
    pub async fn discord_get_channels(
        &self,
        Parameters(DiscordGetChannelsParams { guild_id }): Parameters<DiscordGetChannelsParams>,
    ) -> Result<Json<DiscordGetChannelsResult>, rmcp::ErrorData> {
        use reqwest::Client;
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(guild_id, "Getting Discord channels");

        // Get Discord token
        let token = std::env::var("DISCORD_TOKEN").map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("DISCORD_TOKEN environment variable not set"),
                None,
            )
        })?;

        // Make Discord API request
        let client = Client::new();
        let url = format!("https://discord.com/api/v10/guilds/{}/channels", guild_id);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bot {}", token))
            .header("User-Agent", "Botticelli-MCP/0.2.0")
            .send()
            .await
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Discord API request failed: {}", e)),
                    None,
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Discord API error {}: {}", status, body)),
                None,
            ));
        }

        let channels: Vec<serde_json::Value> = response.json().await.map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Failed to parse Discord response: {}", e)),
                None,
            )
        })?;

        let formatted_channels: Vec<DiscordChannelInfo> = channels
            .into_iter()
            .map(|c| DiscordChannelInfo {
                id: c["id"].as_str().unwrap_or("").to_string(),
                name: c["name"].as_str().map(|s| s.to_string()),
                channel_type: c["type"].as_u64().unwrap_or(0) as u8,
            })
            .collect();

        debug!(count = formatted_channels.len(), "Discord channels retrieved");

        Ok(Json(DiscordGetChannelsResult {
            status: "success".to_string(),
            guild_id,
            count: formatted_channels.len(),
            channels: formatted_channels,
        }))
    }
}

fn default_model() -> String {
    "gemini-2.0-flash-exp".to_string()
}

#[tool_handler]
impl ServerHandler for BotticelliServer {
    fn get_info(&self) -> rmcp::model::InitializeResult {
        rmcp::model::InitializeResult {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "botticelli".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Botticelli".to_string()),
                website_url: None,
                icons: None,
            },
            instructions: Some("Botticelli MCP server - LLM orchestration tools".to_string()),
        }
    }
}

// Helper functions for narrative generation

/// Generate narrative TOML from description.
fn generate_narrative_toml(
    description: &str,
    name: &str,
    default_model: Option<&str>,
    default_temperature: Option<f64>,
) -> Result<String, rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    // Parse description to extract workflow steps
    let acts = NarrativeHelper::extract_acts_from_description(description);

    if acts.is_empty() {
        return Err(rmcp::ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            Cow::Borrowed("Failed to extract acts from description"),
            None,
        ));
    }

    // Build TOML
    let mut toml = String::new();

    // [narrative] section
    toml.push_str("[narrative]\n");
    toml.push_str(&format!("name = \"{}\"\n", name));
    toml.push_str(&format!(
        "description = \"{}\"\n",
        NarrativeHelper::escape_toml_string(description)
    ));

    if let Some(model) = default_model {
        toml.push_str(&format!("model = \"{}\"\n", model));
    }

    if let Some(temp) = default_temperature {
        toml.push_str(&format!("temperature = {}\n", temp));
    }

    toml.push('\n');

    // [toc] section
    toml.push_str("[toc]\n");
    toml.push_str("order = [");
    for (i, act) in acts.iter().enumerate() {
        if i > 0 {
            toml.push_str(", ");
        }
        toml.push_str(&format!("\"{}\"", act.name));
    }
    toml.push_str("]\n\n");

    // [acts] section
    toml.push_str("[acts]\n");
    for act in &acts {
        toml.push_str(&format!(
            "{} = \"{}\"\n",
            act.name,
            NarrativeHelper::escape_toml_string(&act.prompt)
        ));
    }

    Ok(toml)
}

/// Apply modification to narrative TOML.
fn apply_modification(
    toml: &str,
    modification: &str,
) -> Result<(String, Vec<String>), rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    let lower_mod = modification.to_lowercase();
    let mut changes = Vec::new();

    // Detect modification type
    if lower_mod.contains("add act") || lower_mod.contains("add an act") {
        let (modified, change) = add_act(toml, modification)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    if lower_mod.contains("remove act") || lower_mod.contains("delete act") {
        let (modified, change) = remove_act(toml, modification)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    if lower_mod.contains("change model")
        || lower_mod.contains("use model")
        || lower_mod.contains("set model")
        || lower_mod.contains("use gemini")
        || lower_mod.contains("use claude")
        || lower_mod.contains("use gpt")
    {
        let (modified, change) = change_model(toml, modification)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    if lower_mod.contains("temperature") {
        let (modified, change) = change_temperature(toml, modification)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    if lower_mod.contains("add bot") || lower_mod.contains("bot command") {
        let (modified, change) = add_bot_command(toml)?;
        changes.push(change);
        return Ok((modified, changes));
    }

    // Default: try to parse as a general modification
    Err(rmcp::ErrorData::new(
        ErrorCode::INVALID_PARAMS,
        Cow::Owned(format!(
            "Could not understand modification: '{}'. \
             Supported: add/remove act, change model, set temperature, add bot command",
            modification
        )),
        None,
    ))
}

/// Add an act to the narrative.
fn add_act(toml: &str, modification: &str) -> Result<(String, String), rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    // Extract act description (everything after "add act" or "Add act")
    let lower_mod = modification.to_lowercase();
    let desc = if let Some(idx) = lower_mod.find("add act") {
        let after_add_act = &modification[idx + "add act".len()..];
        // Skip "that" if present
        let trimmed = after_add_act.trim();
        if trimmed.starts_with("that") || trimmed.starts_with("which") {
            trimmed
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            trimmed.to_string()
        }
    } else {
        modification.trim().to_string()
    };

    // Generate act name
    let act_name = extract_act_name_from_mod(&desc);

    // Find [toc] and [acts] sections
    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Find TOC line
    let toc_idx = lines
        .iter()
        .position(|line| line.trim().starts_with("order = ["))
        .ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("No [toc] section found"),
                None,
            )
        })?;

    // Add to TOC
    let toc_line = &lines[toc_idx];
    if toc_line.contains(']') {
        let updated_toc = toc_line.replace(']', &format!(", \"{}\"]", act_name));
        lines[toc_idx] = updated_toc;
    }

    // Find [acts] section
    let acts_idx = lines
        .iter()
        .position(|line| line.trim() == "[acts]")
        .ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("No [acts] section found"),
                None,
            )
        })?;

    // Add act definition
    lines.insert(
        acts_idx + 1,
        format!(
            "{} = \"{}\"",
            act_name,
            NarrativeHelper::escape_toml_string(&desc)
        ),
    );

    let modified = lines.join("\n");
    let change = format!("Added act '{}'", act_name);

    Ok((modified, change))
}

/// Remove an act from the narrative.
fn remove_act(toml: &str, modification: &str) -> Result<(String, String), rmcp::ErrorData> {
    // Extract act name to remove - look for the word after "act"
    let lower_mod = modification.to_lowercase();
    let act_name = if let Some(idx) = lower_mod.find("act") {
        // Get text after "act"
        let after_act = &modification[idx + 3..];
        // Skip whitespace and get the next word
        after_act
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_lowercase()
    } else {
        extract_act_name_from_mod(modification)
    };

    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Remove from TOC
    for line in &mut lines {
        if line.trim().starts_with("order = [") {
            *line = line.replace(&format!("\"{}\", ", act_name), "");
            *line = line.replace(&format!(", \"{}\"", act_name), "");
            *line = line.replace(&format!("\"{}\"", act_name), "");
        }
    }

    // Remove act definition
    lines.retain(|line| !line.trim().starts_with(&format!("{} = ", act_name)));

    let modified = lines.join("\n");
    let change = format!("Removed act '{}'", act_name);

    Ok((modified, change))
}

/// Change the model in the narrative.
fn change_model(toml: &str, modification: &str) -> Result<(String, String), rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    // Extract model name
    let model = extract_model_name(modification)?;

    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Find [narrative] section
    let narrative_idx = lines
        .iter()
        .position(|line| line.trim() == "[narrative]")
        .ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("No [narrative] section found"),
                None,
            )
        })?;

    // Find or add model line
    let mut found_model = false;
    for i in (narrative_idx + 1)..lines.len() {
        if lines[i].trim().starts_with("model = ") {
            lines[i] = format!("model = \"{}\"", model);
            found_model = true;
            break;
        }
        if lines[i].trim().starts_with('[') {
            // Next section, insert before it
            lines.insert(i, format!("model = \"{}\"", model));
            found_model = true;
            break;
        }
    }

    if !found_model {
        lines.insert(narrative_idx + 1, format!("model = \"{}\"", model));
    }

    let modified = lines.join("\n");
    let change = format!("Changed model to '{}'", model);

    Ok((modified, change))
}

/// Change the temperature in the narrative.
fn change_temperature(toml: &str, modification: &str) -> Result<(String, String), rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    // Extract temperature value
    let temp = extract_temperature(modification)?;

    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Find [narrative] section
    let narrative_idx = lines
        .iter()
        .position(|line| line.trim() == "[narrative]")
        .ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("No [narrative] section found"),
                None,
            )
        })?;

    // Find or add temperature line
    let mut found_temp = false;
    for i in (narrative_idx + 1)..lines.len() {
        if lines[i].trim().starts_with("temperature = ") {
            lines[i] = format!("temperature = {}", temp);
            found_temp = true;
            break;
        }
        if lines[i].trim().starts_with('[') {
            lines.insert(i, format!("temperature = {}", temp));
            found_temp = true;
            break;
        }
    }

    if !found_temp {
        lines.insert(narrative_idx + 1, format!("temperature = {}", temp));
    }

    let modified = lines.join("\n");
    let change = format!("Changed temperature to {}", temp);

    Ok((modified, change))
}

/// Add a bot command to the narrative.
fn add_bot_command(toml: &str) -> Result<(String, String), rmcp::ErrorData> {
    // Parse bot command details from modification
    let bot_name = "bot_command"; // Simplified for MVP
    let platform = "discord"; // Default

    let mut lines: Vec<String> = toml.lines().map(|s| s.to_string()).collect();

    // Find where to insert [bots] section (before [toc])
    let insert_idx = lines
        .iter()
        .position(|line| line.trim() == "[toc]")
        .unwrap_or(lines.len());

    // Add bot section
    lines.insert(insert_idx, format!("\n[bots.{}]", bot_name));
    lines.insert(insert_idx + 1, format!("platform = \"{}\"", platform));
    lines.insert(
        insert_idx + 2,
        "command = \"server.get_stats\"".to_string(),
    );
    lines.insert(insert_idx + 3, String::new());

    let modified = lines.join("\n");
    let change = format!("Added bot command 'bots.{}'", bot_name);

    Ok((modified, change))
}

/// Extract act name from modification text.
fn extract_act_name_from_mod(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if let Some(verb) = words.first() {
        return verb.to_lowercase();
    }
    "new_act".to_string()
}

/// Extract model name from modification text.
fn extract_model_name(text: &str) -> Result<String, rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    let lower = text.to_lowercase();

    // Common model patterns
    if lower.contains("claude") {
        return Ok("claude-3-5-sonnet-20241022".to_string());
    }
    if lower.contains("gemini") {
        return Ok("gemini-2.0-flash-exp".to_string());
    }
    if lower.contains("gpt-4") {
        return Ok("gpt-4".to_string());
    }
    if lower.contains("gpt") {
        return Ok("gpt-4-turbo".to_string());
    }

    Err(rmcp::ErrorData::new(
        ErrorCode::INVALID_PARAMS,
        Cow::Borrowed("Could not identify model name in modification"),
        None,
    ))
}

/// Extract temperature from modification text.
fn extract_temperature(text: &str) -> Result<f64, rmcp::ErrorData> {
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    let words: Vec<&str> = text.split_whitespace().collect();

    for word in words {
        if let Ok(temp) = word.trim().parse::<f64>() {
            if (0.0..=1.0).contains(&temp) {
                return Ok(temp);
            }
        }
    }

    Err(rmcp::ErrorData::new(
        ErrorCode::INVALID_PARAMS,
        Cow::Borrowed("Could not extract temperature value (must be 0.0-1.0)"),
        None,
    ))
}
