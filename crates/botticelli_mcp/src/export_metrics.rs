//! Export metrics tool types for Prometheus metrics export.

use derive_getters::Getters;
use elicitation::{Prompt, Select};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Output format for metrics export.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    Default,
    elicitation::Elicit,
)]
#[serde(rename_all = "lowercase")]
pub enum MetricsFormat {
    /// Full Prometheus text format
    #[default]
    Prometheus,
    /// Quick statistics summary
    Summary,
}

/// Parameters for exporting metrics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct ExportMetricsParams {
    /// Output format: 'prometheus' for full metrics, 'summary' for quick stats
    #[serde(default)]
    format: MetricsFormat,
}

impl ExportMetricsParams {
    /// Create new export metrics parameters.
    #[tracing::instrument]
    pub fn new(format: MetricsFormat) -> Self {
        Self { format }
    }
}

/// Result from exporting metrics.
///
/// The format field indicates which type of metrics are included.
/// For Prometheus format, the metrics field contains the text.
/// For summary format, the summary fields contain statistics.
#[derive(Debug, Clone, Serialize, JsonSchema, Getters, elicitation::Elicit)]
pub struct ExportMetricsResult {
    /// Output format: "prometheus" or "summary"
    format: String,

    /// Prometheus text format metrics (when format is "prometheus")
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: Option<String>,

    /// Total number of executions (when format is "summary")
    #[serde(skip_serializing_if = "Option::is_none")]
    total_executions: Option<usize>,

    /// Total tokens processed (when format is "summary")
    #[serde(skip_serializing_if = "Option::is_none")]
    total_tokens: Option<u64>,

    /// Total cost in USD (when format is "summary")
    #[serde(skip_serializing_if = "Option::is_none")]
    total_cost_usd: Option<f64>,

    /// Average execution duration in milliseconds (when format is "summary")
    #[serde(skip_serializing_if = "Option::is_none")]
    avg_duration_ms: Option<u64>,
}

impl ExportMetricsResult {
    /// Create a Prometheus format result.
    ///
    /// # Arguments
    ///
    /// * `metrics` - Prometheus text format metrics
    ///
    /// # Returns
    ///
    /// A new `ExportMetricsResult` with Prometheus format.
    #[tracing::instrument(skip(metrics), fields(metrics_len = metrics.len()))]
    pub fn prometheus(metrics: String) -> Self {
        Self {
            format: "prometheus".to_string(),
            metrics: Some(metrics),
            total_executions: None,
            total_tokens: None,
            total_cost_usd: None,
            avg_duration_ms: None,
        }
    }

    /// Create a summary format result.
    ///
    /// # Arguments
    ///
    /// * `total_executions` - Total number of executions
    /// * `total_tokens` - Total tokens processed
    /// * `total_cost_usd` - Total cost in USD
    /// * `avg_duration_ms` - Average execution duration in milliseconds
    ///
    /// # Returns
    ///
    /// A new `ExportMetricsResult` with summary format.
    #[tracing::instrument]
    pub fn summary(
        total_executions: usize,
        total_tokens: u64,
        total_cost_usd: f64,
        avg_duration_ms: u64,
    ) -> Self {
        Self {
            format: "summary".to_string(),
            metrics: None,
            total_executions: Some(total_executions),
            total_tokens: Some(total_tokens),
            total_cost_usd: Some(total_cost_usd),
            avg_duration_ms: Some(avg_duration_ms),
        }
    }
}
