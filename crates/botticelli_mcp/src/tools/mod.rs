//! Tool implementations for MCP server.

mod bot_commands;
mod elicitation;
mod metrics;
mod narrative_creation;
mod narrative_processor;
pub mod narrative_utils;
pub mod narrative_validation_helpers;
mod prometheus;
mod registry;
mod sampling;
mod sampling_session_manager;

pub use bot_commands::{BotCommandRequest, BotCommandResponse};
pub use elicitation::{
    ApplyValidationFixesInput, ApplyValidationFixesOutput,
    ElicitationHelper,
    GetNarrativeStateInput, GetNarrativeStateOutput, NarrativeRegistry,
    PartialNarrativeRegistry, ValidateNarrativeInput, ValidateNarrativeOutput,
};
pub use metrics::{ActMetrics, ExecutionMetrics};
pub use narrative_creation::{
    ElicitActInput, ElicitMetadataInput, FinalizeNarrativeInput, StartNarrativeInput,
    StartNarrativeTool,
};
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
pub use narrative_processor::McpProcessorCollector;
pub use prometheus::{MetricsSummary, PrometheusMetrics};
pub use registry::ToolRegistry;
pub use sampling::{SamplingCoordinator, SamplingHelper, SamplingResult};
pub use sampling_session_manager::SamplingSessionManager;

// Export shared narrative utilities
pub use narrative_utils::{Act, NarrativeHelper};

// Re-export LlmSamplerOperations for convenience
pub use botticelli_interface::LlmSamplerOperations;

