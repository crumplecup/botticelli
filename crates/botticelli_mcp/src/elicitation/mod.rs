//! Elicitation types and traits for narrative construction.

mod dialog;
mod elicitor;
mod partial;
mod registry;

pub use dialog::ElicitationDialog;
pub use elicitor::NarrativeElicitor;
pub use partial::{PartialAct, PartialNarrative, PartialNarrativeBuilder};
// pub use registry::RegistryStorage; // TODO: Remove if unused
