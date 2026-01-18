//! MCP resource system.
//!
//! Resources are data sources that LLMs can read. They follow URI patterns like:
//! - `content://approved_discord_posts/123` - Content by ID
//! - `narrative://curate_content` - Narrative TOML file

#[cfg(feature = "database")]
mod content;
mod narrative;
mod registry;
mod resource_info;

#[cfg(feature = "database")]
pub use content::ContentResource;
pub use narrative::NarrativeResource;
pub use registry::ResourceRegistry;
pub use resource_info::ResourceInfo;
