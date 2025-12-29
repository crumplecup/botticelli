//! Narrative elicitation system.
//!
//! Provides a function-based interface for creating narratives interactively
//! using the elicitation crate's paradigm system (Select, Survey, Affirm) and
//! MCP primitive tools.

mod acts;
mod carousel;
pub mod infrastructure;
mod inputs;
mod metadata;
mod types;
mod validation;

pub use acts::elicit_acts;
pub use carousel::elicit_carousel;
pub use infrastructure::create_mcp_client_for_dialog;
pub use inputs::elicit_inputs;
pub use metadata::elicit_metadata;
pub use types::{
    ActApproach, ActDefinition, BotCommandConfig, CarouselConfig, DocumentInputConfig,
    HistoryRetentionMode, InputType, MediaInputConfig, MediaSource, NarrativeMetadata,
    NarrativeReferenceConfig, OutputFormat, TableQueryConfig, TextInputConfig,
};
pub use validation::elicit_validation;
