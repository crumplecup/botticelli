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
mod session;

#[cfg(feature = "tui")]
mod tui_dialog;

pub use acts::ActElicitor;
pub use dialog::ElicitationDialog;
pub use elicitor::NarrativeElicitor;
pub use metadata::MetadataElicitor;
pub use partial::{PartialAct, PartialNarrative, PartialNarrativeBuilder};
pub use session::ElicitationSession;

#[cfg(feature = "tui")]
pub use tui_dialog::TuiElicitationDialog;
