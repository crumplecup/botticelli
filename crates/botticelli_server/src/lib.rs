//! Inference server and bot runtime for Botticelli.
//!
//! Provides backend-agnostic inference via `Arc<dyn BotticelliDriver>` and ractor-based
//! bot actors (generation, curation, posting). The backend is selected at startup:
//!
//! - `mistral` feature (default): embedded GGUF inference via mistral-rs
//! - `ollama` feature: external Ollama server via `botticelli_models::OllamaClient`

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod actor_traits;
mod api;
mod bots;
mod drivers;
mod metrics;
mod schedule;

pub use actor_traits::{
    ActorManager, ActorServer, ActorServerResult, ContentPoster, StatePersistence, TaskScheduler,
};
#[cfg(feature = "metrics")]
pub use api::{ApiState, create_router as create_metrics_router};
pub use bots::{
    BotServer, CurationBot, CurationBotArgs, CurationMessage, GenerationBot, GenerationBotArgs,
    GenerationMessage, PostingBot, PostingBotArgs, PostingMessage,
};
pub use botticelli_error::{ServerError, ServerErrorKind};
#[cfg(feature = "mistral")]
pub use drivers::{MistralConfig, MistralConfigBuilder, MistralDriver};
#[cfg(feature = "metrics")]
pub use metrics::{
    BotMetrics, MetricsCollector, MetricsSnapshot, NarrativeMetrics, PipelineMetrics, ServerMetrics,
};
pub use schedule::{Schedule, ScheduleCheck, ScheduleType};
