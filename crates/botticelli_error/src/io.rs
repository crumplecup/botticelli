//! I/O error types with source preservation.

#[cfg(feature = "mcp")]
use crate::tool;

use derive_getters::Getters;
use elicitation::Prompt;
use std::sync::Arc;

/// I/O error with source preservation and location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, Getters, elicitation::Elicit)]
#[display("I/O Error at {}:{}", file, line)]
pub struct IoError {
    source: Arc<std::io::Error>,
    line: u32,
    file: String,
}

impl IoError {
    /// Create a new I/O error from a std::io::Error.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(err: std::io::Error) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            source: Arc::new(err),
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }
}

impl From<std::io::Error> for IoError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        Self::new(err)
    }
}

// Manual implementations for traits that io::Error doesn't implement
impl PartialEq for IoError {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line
            && self.file == other.file
            && self.source.kind() == other.source.kind()
    }
}

impl Eq for IoError {}

impl std::hash::Hash for IoError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.line.hash(state);
        self.file.hash(state);
        self.source.kind().hash(state);
    }
}

impl PartialOrd for IoError {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IoError {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.file, self.line, self.source.kind()).cmp(&(
            &other.file,
            other.line,
            other.source.kind(),
        ))
    }
}
