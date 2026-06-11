//! Tool implementations and shared utilities for the MCP server.

#[cfg(feature = "discord")]
pub(crate) mod discord;
pub(crate) mod generate_llm;
mod metrics;
pub(crate) mod modify_narrative;
mod narrative_processor;
pub(crate) mod narrative_utils;
pub(crate) mod narrative_validation_helpers;
mod prometheus;

pub use metrics::{ActMetrics, ExecutionMetrics};
pub use narrative_utils::{Act, NarrativeHelper};
pub use prometheus::{MetricsSummary, PrometheusMetrics};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
pub use narrative_processor::McpProcessorCollector;
