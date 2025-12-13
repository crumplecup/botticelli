//! Narrative elicitation system.
//!
//! Provides a trait-based, conversational interface for creating narratives
//! interactively. Supports multiple UI platforms (TUI, Web, Android) through
//! abstraction.

mod acts;
mod dialog;
mod elicitor;
mod metadata;
mod partial;

pub use acts::ActElicitor;
pub use dialog::ElicitationDialog;
pub use elicitor::NarrativeElicitor;
pub use metadata::MetadataElicitor;
pub use partial::{PartialAct, PartialNarrative, PartialNarrativeBuilder};
