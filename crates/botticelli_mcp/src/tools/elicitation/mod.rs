//! MCP tools for LLM-driven narrative elicitation.

mod carousel;
mod helpers;
mod registry;
mod session_tools;
mod state;
mod update;
mod validation;

pub use carousel::ElicitCarouselTool;
pub use helpers::ElicitationHelper;
pub use registry::NarrativeRegistry;
pub use session_tools::{
    CreateNarrativeSessionTool, ElicitActTool, ElicitMetadataTool, FinalizeNarrativeTool,
};

use crate::PartialNarrative;

/// Type alias for the concrete registry used in elicitation.
pub type PartialNarrativeRegistry = NarrativeRegistry<PartialNarrative>;
