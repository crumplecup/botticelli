//! MCP tools for LLM-driven narrative elicitation.

mod carousel;
mod helpers;
pub mod registry;
mod session_tools;
pub mod state;
mod update;
pub mod validation;

pub use carousel::ElicitCarouselTool;
pub use helpers::ElicitationHelper;
pub use registry::NarrativeRegistry;
pub use session_tools::{
    CreateNarrativeSessionTool, ElicitActTool, ElicitMetadataTool, FinalizeNarrativeTool,
};
pub use state::{GetNarrativeStateInput, GetNarrativeStateOutput};
pub use validation::{
    ApplyValidationFixesInput,
    ApplyValidationFixesOutput, ValidateNarrativeInput, ValidateNarrativeOutput,
};

use crate::PartialNarrative;

/// Type alias for the concrete registry used in elicitation.
pub type PartialNarrativeRegistry = NarrativeRegistry<PartialNarrative>;
