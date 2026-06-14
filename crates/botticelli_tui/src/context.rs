//! Shared read-only context passed to every screen's `handle_key`.
//!
//! Screens never mutate shared state — they receive `&BotScreenContext`
//! and return a `BotTransition` for the controller to act on.

use std::path::PathBuf;
use tracing::instrument;

/// Read-only handles injected into every screen.
///
/// All fields are optional so the TUI degrades gracefully when a backend
/// (e.g. `BotServer`) is not configured.
#[derive(Clone, Default)]
pub struct BotScreenContext {
    /// Directory that holds narrative TOML files.
    pub narratives_dir: Option<PathBuf>,

    /// Path to the active server log file (for the log viewer screen).
    pub log_file: Option<PathBuf>,
}

impl BotScreenContext {
    /// Construct a context with no live services (used in tests).
    #[instrument]
    pub fn mock() -> Self {
        Self::default()
    }
}
