//! Elicitation types and traits for narrative construction.

pub mod dialog;
mod elicitor;
mod partial;

pub use dialog::ElicitationDialog;
pub use elicitor::NarrativeElicitor;
pub use partial::{PartialAct, PartialNarrative, PartialNarrativeBuilder};
