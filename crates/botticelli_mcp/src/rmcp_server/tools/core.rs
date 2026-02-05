//! Core MCP tool implementations.
//!
//! Basic server functionality: echo, server info, content queries, and metrics export.

use crate::rmcp_server::BotticelliServer;
use crate::rmcp_server::helpers::to_mcp_error;
use crate::{
    EchoParams, EchoResult, ExportMetricsParams, ExportMetricsResult, MetricsFormat,
    QueryContentParams, QueryContentResult, ServerInfoResult,
};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::ErrorCode;
use rmcp::tool;
use rmcp::tool_router;
use std::borrow::Cow;
use tracing::{debug, instrument};

#[tool_router(router = core_tool_router, vis = "pub")]
impl BotticelliServer {
    /// Echo back the provided message.
    #[tool]
    #[instrument(skip(self), fields(message))]
    pub async fn echo(
        &self,
        Parameters(params): Parameters<EchoParams>,
    ) -> Result<Json<EchoResult>, rmcp::ErrorData> {
        let message = params.message().clone();
        debug!(?message, "Processing echo request");

        let result = EchoResult::new(message);

        debug!(result = ?result, "Echo completed successfully");
        Ok(Json(result))
    }

    /// Get server information including version and tool count.
    #[tool]
    #[instrument(skip(self))]
    pub async fn server_info(&self) -> Result<Json<ServerInfoResult>, rmcp::ErrorData> {
        debug!("Retrieving server information");

        let result = Json(ServerInfoResult::new(
            "botticelli".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            self.get_tool_router().list_all().len(),
        ));

        debug!(tool_count = result.0.tool_count(), "Server info retrieved");
        Ok(result)
    }

    /// Query content from the database.
    #[tool]
    #[instrument(skip(self, params), fields(table = params.table(), limit = params.limit()))]
    pub async fn query_content(
        &self,
        Parameters(params): Parameters<QueryContentParams>,
    ) -> Result<Json<QueryContentResult>, rmcp::ErrorData> {
        let table = params.table().clone();
        let limit = *params.limit();

        debug!(?table, limit, "Processing query_content request");

        #[cfg(not(feature = "database"))]
        {
            Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Database feature not enabled"),
                None,
            ))
        }

        #[cfg(feature = "database")]
        {
            // Check if database operations are available
            let db_ops = self.db_ops().as_ref().ok_or_else(|| {
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
            let rows: Vec<serde_json::Value> = db_ops
                .execute_query(&query)
                .await
                .map_err(|e| to_mcp_error(e, "Query failed"))?;

            debug!(count = rows.len(), "Retrieved rows from database");

            let result = QueryContentResult::new(table, limit, rows);
            Ok(Json(result))
        }
    }

    /// Export metrics in the requested format.
    #[tool]
    #[instrument(skip(self, params), fields(format = ?params.format()))]
    pub async fn export_metrics(
        &self,
        Parameters(params): Parameters<ExportMetricsParams>,
    ) -> Result<Json<ExportMetricsResult>, rmcp::ErrorData> {
        let format = *params.format();
        debug!(?format, "Exporting metrics");

        // Check if metrics collector is available
        let metrics = self.metrics().as_ref().ok_or_else(|| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("Metrics collector not configured"),
                None,
            )
        })?;

        match format {
            MetricsFormat::Prometheus => {
                let metrics_text = metrics
                    .export_prometheus()
                    .map_err(|e| to_mcp_error(e, "Failed to export Prometheus metrics"))?;

                let result = ExportMetricsResult::prometheus(metrics_text);
                Ok(Json(result))
            }
            MetricsFormat::Summary => {
                let summary = metrics
                    .summary()
                    .map_err(|e| to_mcp_error(e, "Failed to get metrics summary"))?;

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
}
