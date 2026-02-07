//! TOML error types with source preservation.

use rmcp::tool;

use derive_getters::Getters;
use std::sync::Arc;

/// TOML deserialization error with source preservation and location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, Getters)]
#[display("TOML Error at {}:{}", file, line)]
pub struct TomlError {
    source: Arc<toml::de::Error>,
    line: u32,
    file: String,
}

impl TomlError {
    /// Create a new TOML error from a toml::de::Error.
    #[tool]
    #[track_caller]
    pub fn new(err: toml::de::Error) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            source: Arc::new(err),
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }
}

impl From<toml::de::Error> for TomlError {
    #[track_caller]
    fn from(err: toml::de::Error) -> Self {
        Self::new(err)
    }
}

// Manual implementations for traits that toml::de::Error doesn't implement
impl PartialEq for TomlError {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line
            && self.file == other.file
            && format!("{}", self.source) == format!("{}", other.source)
    }
}

impl Eq for TomlError {}

impl std::hash::Hash for TomlError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.line.hash(state);
        self.file.hash(state);
        format!("{}", self.source).hash(state);
    }
}

impl PartialOrd for TomlError {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TomlError {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.file, self.line, format!("{}", self.source)).cmp(&(
            &other.file,
            other.line,
            format!("{}", other.source),
        ))
    }
}
