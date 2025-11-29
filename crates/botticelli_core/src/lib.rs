//! Core data types for the Botticelli LLM API library.
//!
//! This crate provides the foundation data types used across all Botticelli interfaces.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod budget;
mod input;
mod media;
mod message;
mod output;
mod request;
mod role;
mod telemetry;

pub use budget::{BudgetConfig, BudgetConfigBuilder};
pub use input::{HistoryRetention, Input, TableFormat};
pub use media::MediaSource;
pub use message::{Message, MessageBuilder};
pub use output::{Output, ToolCall, ToolCallBuilder};
pub use request::{GenerateRequest, GenerateRequestBuilder, GenerateResponse};
pub use role::Role;
pub use telemetry::{init_telemetry, shutdown_telemetry};
