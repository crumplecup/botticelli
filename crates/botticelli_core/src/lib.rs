//! Core data types for the Botticelli LLM API library.
//!
//! This crate provides the foundation data types used across all Botticelli interfaces.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod budget;
mod input;
mod media;
mod message;
mod observability;
mod output;
mod request;
mod role;

pub use budget::{BudgetConfig, BudgetConfigBuilder};
pub use input::{HistoryRetention, Input, TableFormat};
pub use media::MediaSource;
pub use message::{Message, MessageBuilder};
pub use observability::{
    ExporterBackend, ObservabilityConfig, init_observability, init_observability_with_config,
    shutdown_observability,
};
pub use output::{Output, ToolCall, ToolCallBuilder};
pub use request::{
    GenerateRequest, GenerateRequestBuilder, GenerateResponse, GenerateResponseBuilder,
    GenerateResponseBuilderError,
};
pub use role::Role;
