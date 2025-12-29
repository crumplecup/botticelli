//! Narrative elicitation system.
//!
//! Provides a trait-based, conversational interface for creating narratives
//! interactively. Supports multiple UI platforms (TUI, Web, Android) through
//! abstraction.

mod acts;
mod acts_refactored;
mod carousel;
mod carousel_refactored;
pub mod infrastructure;
mod inputs;
mod inputs_refactored;
mod metadata;
mod metadata_refactored;
mod session;
// Deprecated: DialogTransport has been superseded by InProcTransport + primitive tools
// mod transport;
mod types;
mod validation;
mod validation_refactored;

pub use acts::ActElicitor;
pub use acts_refactored::elicit_acts_refactored;
pub use carousel::CarouselElicitor;
pub use carousel_refactored::elicit_carousel_refactored;
pub use infrastructure::create_mcp_client_for_dialog;
pub use inputs::InputElicitor;
pub use inputs_refactored::elicit_inputs_refactored;
pub use metadata::MetadataElicitor;
pub use metadata_refactored::elicit_metadata_refactored;
pub use session::ElicitationSession;
// Deprecated: Use InProcTransport with primitive elicitation tools instead
// pub use transport::DialogTransport;
pub use types::{
    ActApproach, ActDefinition, BotCommandConfig, CarouselConfig, DocumentInputConfig,
    HistoryRetentionMode, InputType, MediaInputConfig, MediaSource, NarrativeMetadata,
    NarrativeReferenceConfig, OutputFormat, TableQueryConfig, TextInputConfig,
};
pub use validation::ValidationElicitor;
pub use validation_refactored::elicit_validation_refactored;
