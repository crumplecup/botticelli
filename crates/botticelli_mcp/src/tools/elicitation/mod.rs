//! MCP tools for LLM-driven narrative elicitation.

mod carousel;
mod helpers;
mod registry;
mod session_tools;
mod state;
mod update;
mod validation;

pub use carousel::{CarouselLevel, ElicitCarouselInput, ElicitCarouselOutput};
pub use helpers::ElicitationHelper;
pub use registry::NarrativeRegistry;
pub use session_tools::{
    CreateNarrativeSessionTool, ElicitActTool, ElicitMetadataTool, FinalizeNarrativeTool,
};
pub use state::{GetNarrativeStateInput, GetNarrativeStateOutput, StateFormat};
pub use update::{FieldUpdate, UpdateNarrativeFieldInput, UpdateNarrativeFieldOutput};
pub use validation::{ValidateNarrativeInput, ValidateNarrativeOutput};

use crate::PartialNarrative;

/// Type alias for the concrete registry used in elicitation.
pub type PartialNarrativeRegistry = NarrativeRegistry<PartialNarrative>;
