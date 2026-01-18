//! Observability metrics for MCP tools.
//!
//! Provides token counting, cost calculation, and execution time tracking
//! for narrative executions and LLM interactions.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, instrument};

/// Metrics collected during narrative execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Total input tokens consumed
    pub input_tokens: u64,
    /// Total output tokens generated
    pub output_tokens: u64,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Execution duration
    pub duration_ms: u64,
    /// Per-act breakdown
    pub act_metrics: Vec<ActMetrics>,
}

/// Metrics for a single act execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActMetrics {
    /// Act name
    pub act_name: String,
    /// Model used
    pub model: String,
    /// Input tokens
    pub input_tokens: u64,
    /// Output tokens
    pub output_tokens: u64,
    /// Cost in USD
    pub cost_usd: f64,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl ExecutionMetrics {
    /// Create new empty metrics.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            total_cost_usd: 0.0,
            duration_ms: 0,
            act_metrics: Vec::new(),
        }
    }

    /// Add metrics from a single act execution.
    #[instrument(skip(self))]
    pub fn add_act(&mut self, metrics: ActMetrics) {
        debug!(
            act_name = %metrics.act_name,
            input_tokens = metrics.input_tokens,
            output_tokens = metrics.output_tokens,
            cost_usd = metrics.cost_usd,
            "Recording act metrics"
        );

        self.input_tokens += metrics.input_tokens;
        self.output_tokens += metrics.output_tokens;
        self.total_cost_usd += metrics.cost_usd;
        self.act_metrics.push(metrics);
    }

    /// Set total execution duration.
    #[tracing::instrument(skip(self))]
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration_ms = duration.as_millis() as u64;
    }

    /// Get total token count.
    #[tracing::instrument(skip(self))]
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

impl Default for ExecutionMetrics {
    #[tracing::instrument]
    fn default() -> Self {
        Self::new()
    }
}

impl ActMetrics {
    /// Create new act metrics.
    #[tracing::instrument(skip(act_name, model), fields(act = %act_name, model = %model))]
    pub fn new(
        act_name: String,
        model: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        duration: Duration,
    ) -> Self {
        Self {
            act_name,
            model,
            input_tokens,
            output_tokens,
            cost_usd,
            duration_ms: duration.as_millis() as u64,
        }
    }
}
