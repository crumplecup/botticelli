//! RMCP-based MCP server implementation.
//!
//! The BotticelliServer struct holds all tool implementations and state
//! needed for MCP operations.

use crate::dialog_resource::DialogResource;
use crate::{
    EchoParams, EchoResult, ElicitBoolParams, ElicitBoolResult, ElicitNumberParams,
    ElicitNumberResult, ElicitSelectParams, ElicitSelectResult, ElicitTextParams,
    ElicitTextResult, ExportMetricsParams, ExportMetricsResult, MetricsFormat, PrometheusMetrics,
    QueryContentParams, QueryContentResult, ServerInfoResult,
};
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
        }
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
