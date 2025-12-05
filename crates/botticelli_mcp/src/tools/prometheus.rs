//! Prometheus metrics export for MCP server.
//!
//! Exports execution metrics in Prometheus format for monitoring dashboards.

use crate::tools::metrics::ExecutionMetrics;
use std::sync::{Arc, Mutex};
use tracing::{debug, instrument};

/// Prometheus metrics collector for MCP operations.
#[derive(Debug, Clone)]
pub struct PrometheusMetrics {
    executions: Arc<Mutex<Vec<ExecutionMetrics>>>,
}

impl PrometheusMetrics {
    /// Create a new Prometheus metrics collector.
    pub fn new() -> Self {
        Self {
            executions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record execution metrics.
    #[instrument(skip(self, metrics))]
    pub fn record_execution(&self, metrics: ExecutionMetrics) {
        debug!(
            total_tokens = metrics.total_tokens(),
            cost_usd = metrics.total_cost_usd,
            duration_ms = metrics.duration_ms,
            "Recording execution metrics"
        );

        if let Ok(mut executions) = self.executions.lock() {
            executions.push(metrics);
        }
    }

    /// Export metrics in Prometheus text format.
    #[instrument(skip(self))]
    pub fn export_prometheus(&self) -> String {
        let executions = self.executions.lock().unwrap();

        let mut output = String::new();

        // Total executions
        output.push_str("# HELP mcp_narrative_executions_total Total narrative executions\n");
        output.push_str("# TYPE mcp_narrative_executions_total counter\n");
        output.push_str(&format!(
            "mcp_narrative_executions_total {}\n",
            executions.len()
        ));

        // Total tokens
        let total_tokens: u64 = executions.iter().map(|e| e.total_tokens()).sum();
        output.push_str("# HELP mcp_tokens_total Total tokens processed\n");
        output.push_str("# TYPE mcp_tokens_total counter\n");
        output.push_str(&format!("mcp_tokens_total {}\n", total_tokens));

        // Input tokens
        let input_tokens: u64 = executions.iter().map(|e| e.input_tokens).sum();
        output.push_str("# HELP mcp_input_tokens_total Total input tokens\n");
        output.push_str("# TYPE mcp_input_tokens_total counter\n");
        output.push_str(&format!("mcp_input_tokens_total {}\n", input_tokens));

        // Output tokens
        let output_tokens: u64 = executions.iter().map(|e| e.output_tokens).sum();
        output.push_str("# HELP mcp_output_tokens_total Total output tokens\n");
        output.push_str("# TYPE mcp_output_tokens_total counter\n");
        output.push_str(&format!("mcp_output_tokens_total {}\n", output_tokens));

        // Total cost
        let total_cost: f64 = executions.iter().map(|e| e.total_cost_usd).sum();
        output.push_str("# HELP mcp_cost_usd_total Total cost in USD\n");
        output.push_str("# TYPE mcp_cost_usd_total counter\n");
        output.push_str(&format!("mcp_cost_usd_total {}\n", total_cost));

        // Average execution time
        if !executions.is_empty() {
            let avg_duration: u64 =
                executions.iter().map(|e| e.duration_ms).sum::<u64>() / executions.len() as u64;
            output.push_str("# HELP mcp_execution_duration_ms_avg Average execution duration\n");
            output.push_str("# TYPE mcp_execution_duration_ms_avg gauge\n");
            output.push_str(&format!("mcp_execution_duration_ms_avg {}\n", avg_duration));
        }

        // Per-model metrics
        let mut model_tokens: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        let mut model_costs: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();

        for execution in executions.iter() {
            for act in &execution.act_metrics {
                *model_tokens.entry(act.model.clone()).or_insert(0) +=
                    act.input_tokens + act.output_tokens;
                *model_costs.entry(act.model.clone()).or_insert(0.0) += act.cost_usd;
            }
        }

        output.push_str("# HELP mcp_model_tokens_total Tokens per model\n");
        output.push_str("# TYPE mcp_model_tokens_total counter\n");
        for (model, tokens) in model_tokens {
            output.push_str(&format!(
                "mcp_model_tokens_total{{model=\"{}\"}} {}\n",
                model, tokens
            ));
        }

        output.push_str("# HELP mcp_model_cost_usd_total Cost per model in USD\n");
        output.push_str("# TYPE mcp_model_cost_usd_total counter\n");
        for (model, cost) in model_costs {
            output.push_str(&format!(
                "mcp_model_cost_usd_total{{model=\"{}\"}} {}\n",
                model, cost
            ));
        }

        debug!(metrics_size = output.len(), "Exported Prometheus metrics");
        output
    }

    /// Get summary statistics.
    #[instrument(skip(self))]
    pub fn summary(&self) -> MetricsSummary {
        let executions = self.executions.lock().unwrap();

        let total_executions = executions.len();
        let total_tokens: u64 = executions.iter().map(|e| e.total_tokens()).sum();
        let total_cost: f64 = executions.iter().map(|e| e.total_cost_usd).sum();
        let avg_duration = if !executions.is_empty() {
            executions.iter().map(|e| e.duration_ms).sum::<u64>() / executions.len() as u64
        } else {
            0
        };

        MetricsSummary {
            total_executions,
            total_tokens,
            total_cost_usd: total_cost,
            avg_duration_ms: avg_duration,
        }
    }
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of execution metrics.
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    /// Total number of executions
    pub total_executions: usize,
    /// Total tokens processed
    pub total_tokens: u64,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Average execution duration in milliseconds
    pub avg_duration_ms: u64,
}
