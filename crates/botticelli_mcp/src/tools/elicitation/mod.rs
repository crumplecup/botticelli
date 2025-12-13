//! MCP tools for LLM-driven narrative elicitation.

mod registry;
mod session;

pub use registry::NarrativeRegistry;
pub use session::CreateNarrativeSessionTool;
