//! MCP tools for LLM-driven narrative elicitation.

mod helpers;
pub mod registry;
pub mod state;
pub mod validation;

pub use helpers::ElicitationHelper;
pub use registry::NarrativeRegistry;
pub use state::{GetNarrativeStateInput, GetNarrativeStateOutput};
pub use validation::{
    ApplyValidationFixesInput, ApplyValidationFixesOutput, ValidateNarrativeInput,
    ValidateNarrativeOutput,
};

use crate::PartialNarrative;

/// Type alias for the concrete registry used in elicitation.
pub type PartialNarrativeRegistry = NarrativeRegistry<PartialNarrative>;
