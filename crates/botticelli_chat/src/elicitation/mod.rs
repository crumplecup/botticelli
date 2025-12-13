//! Narrative elicitation system.
//!
//! Provides a trait-based, conversational interface for creating narratives
//! interactively. Supports multiple UI platforms (TUI, Web, Android) through
//! abstraction.

mod acts;
mod carousel;
mod dialog;
mod elicitor;
mod inputs;
mod metadata;
mod partial;
mod session;
mod validation;

#[cfg(feature = "tui")]
mod tui_dialog;

pub use acts::ActElicitor;
pub use carousel::CarouselElicitor;
pub use dialog::ElicitationDialog;
pub use elicitor::NarrativeElicitor;
pub use inputs::InputElicitor;
pub use metadata::MetadataElicitor;
pub use partial::{PartialAct, PartialNarrative, PartialNarrativeBuilder};
pub use session::ElicitationSession;
pub use validation::ValidationElicitor;

#[cfg(feature = "tui")]
pub use tui_dialog::TuiElicitationDialog;
