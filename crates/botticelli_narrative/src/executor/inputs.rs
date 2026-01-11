//! Input processing methods for narrative execution.

use super::core::NarrativeExecutor;
use super::utils;
use crate::{ActConfig, CarouselConfig, NarrativeMetadata};
use botticelli_core::{ActExecution, Input};
use botticelli_error::{BotticelliResult, NarrativeError, NarrativeErrorKind};
use botticelli_interface::{BotticelliDriver, NarrativeProvider};
use botticelli_rate_limit::TierConfig;
use serde_json::Value as JsonValue;
use tracing::instrument;

/// Parameters for table query processing.
#[derive(Debug)]
pub(super) struct TableQueryParams<'a> {
    pub table_name: &'a str,
    pub columns: Option<&'a Vec<String>>,
    pub where_clause: Option<&'a str>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<&'a str>,
    pub format: botticelli_core::TableFormat,
}

impl<D, BE> NarrativeExecutor<D, BE>
where
    D: BotticelliDriver<
            Request = botticelli_core::GenerateRequest,
            Response = botticelli_core::GenerateResponse,
        >,
    BE: std::error::Error + Send + Sync + 'static,
{
    /// Process a bot command input, executing it and capturing IDs to state.
    #[instrument(skip(self, args, act_executions), fields(platform, command))]
    pub(super) async fn process_bot_command(
        &self,
        platform: &str,
        command: &str,
        args: &std::collections::HashMap<String, JsonValue>,
        required: bool,
        act_executions: &[ActExecution],
        current_index: usize,
    ) -> BotticelliResult<(Input, Option<JsonValue>)> {
        tracing::debug!(
            platform = %platform,
            command = %command,
            required = %required,
            "Processing bot command input"
        );

        let registry = self.bot_registry.as_ref().ok_or_else(|| {
            let msg = format!(
                "Bot command '{}' requires bot_registry to be configured",
                command
            );
            tracing::error!(platform = %platform, command = %command, msg);
            NarrativeError::new(NarrativeErrorKind::BotCommandNotConfigured(msg))
        })?;

        // Resolve templates in bot command arguments
        let mut resolved_args = args.clone();
        for (_key, value) in resolved_args.iter_mut() {
            if let JsonValue::String(s) = value {
                *s = utils::resolve_template(
                    s,
                    act_executions,
                    current_index,
                    self.state_manager.as_ref(),
                )?;
            }
        }

        match registry.execute(platform, command, &resolved_args).await {
            Ok(result) => {
                // Extract and save IDs to state if state_manager is available
                if let Some(state_mgr) = &self.state_manager {
                    utils::capture_bot_command_ids(state_mgr, platform, command, &result)?;
                }

                // Convert JSON result to pretty-printed text for LLM context
                let text = serde_json::to_string_pretty(&result).map_err(|e| {
                    tracing::error!(error = %e, "Failed to serialize bot command result");
                    NarrativeError::new(NarrativeErrorKind::SerializationError(format!(
                        "Bot command result serialization failed: {}",
                        e
                    )))
                })?;

                tracing::info!(
                    platform = %platform,
                    command = %command,
                    result_length = text.len(),
                    "Bot command executed successfully"
                );

                Ok((Input::Text(text), Some(result)))
            }
            Err(e) => {
                if required {
                    tracing::error!(
                        platform = %platform,
                        command = %command,
                        error = %e,
                        "Required bot command failed, halting execution"
                    );
                    Err(
                        NarrativeError::new(NarrativeErrorKind::BotCommandFailed(format!(
                            "Required command '{}' failed: {}",
                            command, e
                        )))
                        .into(),
                    )
                } else {
                    tracing::warn!(
                        platform = %platform,
                        command = %command,
                        error = %e,
                        "Optional bot command failed, continuing with error message"
                    );
                    let error_msg = format!("[Bot command '{}' failed: {}]", command, e);
                    Ok((Input::Text(error_msg), None))
                }
            }
        }
    }

    /// Process a table query input by building and executing the query.
    /// Returns Some(Input::Text) if successful, None if table not found (treated as optional).
    #[instrument(skip(self, params), fields(table_name = %params.table_name))]
    pub(super) async fn process_table_input(
        &self,
        params: TableQueryParams<'_>,
    ) -> BotticelliResult<Option<Input>> {
        let format_str = match params.format {
            botticelli_core::TableFormat::Json => "json",
            botticelli_core::TableFormat::Markdown => "markdown",
            botticelli_core::TableFormat::Csv => "csv",
        };

        tracing::debug!(
            table_name = %params.table_name,
            format = %format_str,
            "Processing table reference input"
        );

        let registry = self.table_registry.as_ref().ok_or_else(|| {
            let msg = format!(
                "Table reference '{}' requires table_registry to be configured",
                params.table_name
            );
            tracing::error!(table_name = %params.table_name, msg);
            NarrativeError::new(NarrativeErrorKind::TableQueryNotConfigured(msg))
        })?;

        // Build query view
        let mut query_builder = botticelli_database::TableQueryViewBuilder::default();
        query_builder.table_name(params.table_name.to_string());

        if let Some(cols) = params.columns {
            query_builder.columns(cols.clone());
        }
        if let Some(where_str) = params.where_clause {
            query_builder.filter(where_str.to_string());
        }
        if let Some(lim) = params.limit {
            query_builder.limit(lim as i64);
        }
        if let Some(off) = params.offset {
            query_builder.offset(off as i64);
        }
        if let Some(order) = params.order_by {
            query_builder.order_by(order.to_string());
        }
        query_builder.format(format_str.to_string());

        let query_view = query_builder.build().map_err(|e| {
            NarrativeError::new(NarrativeErrorKind::TableQueryNotConfigured(format!(
                "Failed to build query: {}",
                e
            )))
        })?;

        match registry.query_table(&query_view).await {
            Ok(result) => {
                tracing::info!(
                    table_name = %params.table_name,
                    result_length = result.len(),
                    "Table query executed successfully"
                );
                Ok(Some(Input::Text(result)))
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("not found") {
                    tracing::warn!(
                        table_name = %params.table_name,
                        "Table not found, treating as empty result for optional input"
                    );
                    Ok(None)
                } else {
                    tracing::error!(
                        table_name = %params.table_name,
                        error = %e,
                        "Table query failed"
                    );
                    Err(
                        NarrativeError::new(NarrativeErrorKind::TableQueryFailed(format!(
                            "Table query '{}' failed: {}",
                            params.table_name, e
                        )))
                        .into(),
                    )
                }
            }
        }
    }

    /// Process a nested narrative input by loading and executing it.
    #[instrument(skip(self, narrative), fields(narrative_name = %name))]
    pub(super) async fn process_narrative_input<N>(
        &self,
        narrative: &N,
        name: &str,
        path: Option<&str>,
    ) -> BotticelliResult<()>
    where
        N: NarrativeProvider<
                Metadata = NarrativeMetadata,
                ActConfig = ActConfig,
                CarouselConfig = CarouselConfig,
            > + ?Sized,
    {
        tracing::debug!(
            name = %name,
            path = ?path,
            "Processing narrative reference input - executing nested narrative"
        );

        // Resolve the path (if None, use name.toml)
        let mut narrative_path = path.unwrap_or(name).to_string();
        if !narrative_path.ends_with(".toml") {
            narrative_path.push_str(".toml");
        }

        // Resolve path relative to parent narrative's directory
        let resolved_path = if std::path::Path::new(&narrative_path).is_absolute() {
            std::path::PathBuf::from(&narrative_path)
        } else if let Some(parent_path) = narrative.source_path() {
            parent_path
                .parent()
                .map(|p| p.join(&narrative_path))
                .unwrap_or_else(|| std::path::PathBuf::from(&narrative_path))
        } else {
            std::path::PathBuf::from(&narrative_path)
        };

        tracing::info!(
            name = %name,
            path = %resolved_path.display(),
            exists = %resolved_path.exists(),
            "Loading nested narrative"
        );

        // Load the nested narrative from file
        let nested_narrative = crate::Narrative::from_file(&resolved_path).map_err(|e| {
            tracing::error!(
                name = %name,
                path = %resolved_path.display(),
                error = %e,
                "Failed to load nested narrative"
            );
            NarrativeError::new(NarrativeErrorKind::NestedNarrativeLoadFailed(format!(
                "Failed to load nested narrative '{}' from '{}': {}",
                name,
                resolved_path.display(),
                e
            )))
        })?;

        tracing::info!(
            name = %name,
            acts = nested_narrative.act_names().len(),
            "Executing nested narrative"
        );

        // Execute the nested narrative recursively
        let nested_execution = Box::pin(self.execute(&nested_narrative))
            .await
            .map_err(|e| {
                tracing::error!(
                    name = %name,
                    error = %e,
                    "Nested narrative execution failed"
                );
                NarrativeError::new(NarrativeErrorKind::NestedNarrativeExecutionFailed(format!(
                    "Nested narrative '{}' execution failed: {}",
                    name, e
                )))
            })?;

        tracing::info!(
            name = %name,
            acts_executed = nested_execution.act_executions().len(),
            "Nested narrative execution completed"
        );

        Ok(())
    }

    #[tracing::instrument(
        skip(self, narrative, inputs, act_executions),
        fields(
            narrative_name = narrative.name(),
            input_count = inputs.len(),
            act_index = current_index,
            bot_commands = 0,
            tables = 0
        )
    )]
    pub(super) async fn process_inputs<N>(
        &self,
        narrative: &N,
        inputs: &[Input],
        act_executions: &[ActExecution],
        current_index: usize,
    ) -> BotticelliResult<(Vec<Input>, Option<JsonValue>)>
    where
        N: NarrativeProvider<
                Metadata = NarrativeMetadata,
                ActConfig = ActConfig,
                CarouselConfig = CarouselConfig,
            > + ?Sized,
    {
        tracing::debug!("Processing inputs for act");
        let mut processed = Vec::new();
        let mut bot_command_count = 0;
        let mut table_count = 0;
        let mut last_bot_command_result: Option<JsonValue> = None;

        for input in inputs {
            match input {
                Input::BotCommand {
                    platform,
                    command,
                    args,
                    required,
                    ..
                } => {
                    bot_command_count += 1;
                    let (result_input, result_json) = self
                        .process_bot_command(
                            platform,
                            command,
                            args,
                            *required,
                            act_executions,
                            current_index,
                        )
                        .await?;
                    processed.push(result_input);
                    if let Some(json) = result_json {
                        last_bot_command_result = Some(json);
                    }
                }

                Input::Table {
                    table_name,
                    columns,
                    where_clause,
                    limit,
                    offset,
                    order_by,
                    format,
                    ..
                } => {
                    table_count += 1;
                    if let Some(result) = self
                        .process_table_input(TableQueryParams {
                            table_name,
                            columns: columns.as_ref(),
                            where_clause: where_clause.as_deref(),
                            limit: *limit,
                            offset: *offset,
                            order_by: order_by.as_deref(),
                            format: *format,
                        })
                        .await?
                    {
                        processed.push(result);
                    }
                }

                Input::Narrative { name, path, .. } => {
                    self.process_narrative_input(narrative, name, path.as_deref())
                        .await?;
                }

                Input::Text(text) => {
                    let resolved = utils::resolve_template(
                        text,
                        act_executions,
                        current_index,
                        self.state_manager.as_ref(),
                    )?;
                    processed.push(Input::Text(resolved));
                }

                // Pass through all other input types unchanged
                other => {
                    processed.push(other.clone());
                }
            }
        }

        tracing::Span::current().record("bot_commands", bot_command_count);
        tracing::Span::current().record("tables", table_count);

        Ok((processed, last_bot_command_result))
    }
}
