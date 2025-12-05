//! Tool for exporting Prometheus metrics.

use crate::tools::McpTool;
use crate::{McpError, McpResult, PrometheusMetrics};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, instrument};

/// MCP tool for exporting execution metrics in Prometheus format.
pub struct ExportMetricsTool {
    metrics: Arc<PrometheusMetrics>,
}

impl ExportMetricsTool {
    /// Creates a new export metrics tool.
    pub fn new(metrics: Arc<PrometheusMetrics>) -> Self {
        Self { metrics }
    }
}

#[async_trait]
impl McpTool for ExportMetricsTool {
    fn name(&self) -> &str {
        "export_metrics"
    }

    fn description(&self) -> &str {
        "Export execution metrics in Prometheus text format for monitoring dashboards"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "enum": ["prometheus", "summary"],
                    "description": "Output format: 'prometheus' for full metrics, 'summary' for quick stats",
                    "default": "prometheus"
                }
            }
        })
    }

    #[instrument(skip(self, input))]
    async fn execute(&self, input: Value) -> McpResult<Value> {
        let format = input
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("prometheus");

        debug!(format = %format, "Exporting metrics");

        match format {
            "prometheus" => {
                let metrics_text = self.metrics.export_prometheus();
                Ok(json!({
                    "format": "prometheus",
                    "metrics": metrics_text
                }))
            }
            "summary" => {
                let summary = self.metrics.summary();
                Ok(json!({
                    "format": "summary",
                    "total_executions": summary.total_executions,
                    "total_tokens": summary.total_tokens,
                    "total_cost_usd": summary.total_cost_usd,
                    "avg_duration_ms": summary.avg_duration_ms
                }))
            }
            _ => Err(McpError::InvalidInput(format!(
                "Unknown format: {}",
                format
            ))),
        }
    }
}
