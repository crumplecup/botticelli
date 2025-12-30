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
    CreateNarrativeParams, CreateNarrativeResult, CreateSceneParams, CreateSceneResult,
    DeleteSceneParams, DeleteSceneResult, EchoParams, EchoResult, ElicitBoolParams,
    ElicitBoolResult, ElicitNumberParams, ElicitNumberResult, ElicitSelectParams,
    ElicitSelectResult, ElicitTextParams, ElicitTextResult, ExportMetricsParams,
    ExportMetricsResult, ListScenesParams, ListScenesResult, MetricsFormat,
    ModifyNarrativeParams, ModifyNarrativeResult, PrometheusMetrics, QueryContentParams,
    QueryContentResult, SaveNarrativeParams, SaveNarrativeResult, ServerInfoResult,
    UpdateSceneParams, UpdateSceneResult,
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
