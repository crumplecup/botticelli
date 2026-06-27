//! Shared read-only context passed to every screen's `handle_key`.
//!
//! Screens never mutate shared state — they receive `&BotScreenContext`
//! and return a `BotTransition` for the controller to act on.

use std::path::PathBuf;
use std::sync::Arc;
use tracing::instrument;

use botticelli_interface::BotStorage;

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

    /// Storage backend for the database browser and schedule history screens.
    ///
    /// `None` when storage is not configured — affected screens degrade
    /// gracefully with a "storage not configured" message.
    pub storage: Option<Arc<dyn BotStorage>>,
}

impl BotScreenContext {
    /// Construct a context with no live services (used in tests).
    #[instrument]
    pub fn mock() -> Self {
        Self::default()
    }
}
