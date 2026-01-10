//! TOML deserialization structures for narrative configuration.

mod act;
mod definitions;
mod file;
mod input;
mod narrative;
mod utils;

pub use act::TomlAct;
pub use definitions::TomlNarrativeReference;
pub use file::{TomlNarrativeData, TomlNarrativeFile};
pub use input::TomlInput;
pub use narrative::TomlNarrativeDefinition;
