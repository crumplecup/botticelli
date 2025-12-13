//! Narrative elicitation system.
//!
//! Provides a trait-based, conversational interface for creating narratives
//! interactively. Supports multiple UI platforms (TUI, Web, Android) through
//! abstraction.

mod dialog;
mod elicitor;
mod partial;

pub use dialog::ElicitationDialog;
pub use elicitor::NarrativeElicitor;
pub use partial::{PartialAct, PartialNarrative, PartialNarrativeBuilder};
