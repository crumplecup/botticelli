//! Narrative elicitation system.
//!
//! Provides a trait-based, conversational interface for creating narratives
//! interactively. Supports multiple UI platforms (TUI, Web, Android) through
//! abstraction.

mod acts;
mod carousel;
mod inputs;
mod metadata;
mod session;
mod types;
mod validation;

pub use acts::ActElicitor;
pub use carousel::CarouselElicitor;
pub use inputs::InputElicitor;
pub use metadata::MetadataElicitor;
pub use session::ElicitationSession;
pub use types::{
    ActApproach, CarouselConfig, HistoryRetentionMode, InputType, MediaSource,
    NarrativeMetadata, OutputFormat,
};
pub use validation::ValidationElicitor;
