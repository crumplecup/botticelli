//! I/O error types with source preservation.

use derive_getters::Getters;

/// I/O error with source preservation and location tracking.
#[derive(Debug, derive_more::Display, derive_more::Error, Getters)]
#[display("I/O Error at {}:{}", file, line)]
pub struct IoError {
    source: Box<std::io::Error>,
    line: u32,
    file: &'static str,
}

impl IoError {
    /// Create a new I/O error from a std::io::Error.
    #[track_caller]
    pub fn new(err: std::io::Error) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            source: Box::new(err),
            line: loc.line(),
            file: loc.file(),
        }
    }
}

impl From<std::io::Error> for IoError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        Self::new(err)
    }
}
