use prometheus::{
    register_counter_vec_with_registry, register_histogram_vec_with_registry, CounterVec,
    HistogramVec, Registry,
};
use std::sync::Arc;
use tracing::instrument;

use crate::McpClientResult;

/// Prometheus metrics for MCP client operations.
#[derive(Debug, Clone)]
pub struct McpClientMetrics {
    /// Tool call counts by tool name and status (success/failure).
    tool_calls: Arc<CounterVec>,

    /// Tool execution duration in seconds.
    tool_duration: Arc<HistogramVec>,

    /// Tokens used per conversation turn (input/output).
    tokens_per_turn: Arc<HistogramVec>,

    /// Cost per workflow in USD.
    workflow_cost: Arc<HistogramVec>,

    /// Agent iterations per workflow.
    agent_iterations: Arc<HistogramVec>,
}

impl McpClientMetrics {
    /// Creates new MCP client metrics registered with the given registry.
    #[instrument(skip(registry))]
    pub fn new(registry: &Registry) -> McpClientResult<Self> {
        let tool_calls = register_counter_vec_with_registry!(
            "mcp_client_tool_calls_total",
            "Total number of MCP tool calls",
            &["tool_name", "status"],
            registry
        )?;

        let tool_duration = register_histogram_vec_with_registry!(
            "mcp_client_tool_duration_seconds",
            "MCP tool execution duration in seconds",
            &["tool_name"],
            registry
        )?;

        let tokens_per_turn = register_histogram_vec_with_registry!(
            "mcp_client_tokens_per_turn",
            "Tokens used per conversation turn",
            &["direction"], // input/output
            registry
        )?;

        let workflow_cost = register_histogram_vec_with_registry!(
            "mcp_client_workflow_cost_usd",
            "Cost per workflow in USD",
            &["model"],
            registry
        )?;

        let agent_iterations = register_histogram_vec_with_registry!(
            "mcp_client_agent_iterations",
            "Number of agent iterations per workflow",
            &["status"], // completed/max_iterations/error
            registry
        )?;

        Ok(Self {
            tool_calls: Arc::new(tool_calls),
            tool_duration: Arc::new(tool_duration),
            tokens_per_turn: Arc::new(tokens_per_turn),
            workflow_cost: Arc::new(workflow_cost),
            agent_iterations: Arc::new(agent_iterations),
        })
    }

    /// Records a tool call.
    #[instrument(skip(self))]
    pub fn record_tool_call(&self, tool_name: &str, success: bool) {
        let status = if success { "success" } else { "failure" };
        self.tool_calls
            .with_label_values(&[tool_name, status])
            .inc();
    }

    /// Records tool execution duration.
    #[instrument(skip(self))]
    pub fn record_tool_duration(&self, tool_name: &str, duration_secs: f64) {
        self.tool_duration
            .with_label_values(&[tool_name])
            .observe(duration_secs);
    }

    /// Records tokens used in a conversation turn.
    #[instrument(skip(self))]
    pub fn record_tokens(&self, input_tokens: u64, output_tokens: u64) {
        self.tokens_per_turn
            .with_label_values(&["input"])
            .observe(input_tokens as f64);
        self.tokens_per_turn
            .with_label_values(&["output"])
            .observe(output_tokens as f64);
    }

    /// Records workflow cost.
    #[instrument(skip(self))]
    pub fn record_workflow_cost(&self, model: &str, cost_usd: f64) {
        self.workflow_cost
            .with_label_values(&[model])
            .observe(cost_usd);
    }

    /// Records agent iterations.
    #[instrument(skip(self))]
    pub fn record_agent_iterations(&self, iterations: usize, status: &str) {
        self.agent_iterations
            .with_label_values(&[status])
            .observe(iterations as f64);
    }
}
