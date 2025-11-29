//! Bot server for orchestrating automated content generation, curation, and posting.
//!
//! This crate provides three independent bot actors that work together:
//! - **GenerationBot**: Creates new content via narrative execution
//! - **CurationBot**: Reviews and approves generated content  
//! - **PostingBot**: Posts approved content to Discord with jitter

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod api;
mod config;
mod curation;
mod generation;
mod metrics;
mod posting;
mod server;

pub use api::{ApiState, create_router};
pub use config::{BotConfig, BotSchedule, CurationConfig, GenerationConfig, PostingConfig};
pub use curation::{CurationBot, CurationMessage};
pub use generation::{GenerationBot, GenerationMessage};
pub use metrics::{BotMetricSnapshot, BotMetrics, MetricsSnapshot};
pub use posting::{PostingBot, PostingMessage};
pub use server::BotServer;
