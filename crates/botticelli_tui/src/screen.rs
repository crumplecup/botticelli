//! `BotScreen` trait and `BotTransition` state-machine enum.

use crossterm::event::KeyEvent;
use elicit_ratatui::TuiNode;
use std::path::PathBuf;

use crate::context::BotScreenContext;

/// Result of one key-event on a screen.
///
/// Screens return a `BotTransition`; [`BotController`](crate::controller::BotController)
/// applies it — screens never mutate shared state or do I/O directly.
#[derive(Debug, Clone)]
pub enum BotTransition {
    /// No change — redraw with same screen.
    Stay,
    /// Navigate to the bot operator console.
    GoToBots,
    /// Navigate to the chat screen.
    GoToChat,
    /// Navigate to the narrative file browser.
    GoToNarratives,
    /// Open the narrative editor, optionally pre-loading a file.
    GoToNarrativeEditor {
        /// Path of the file to load, or `None` to create a blank narrative.
        path: Option<PathBuf>,
    },
    /// Navigate to the database browser.
    GoToDatabase,
    /// Navigate to the schedule view.
    GoToSchedule,
    /// Navigate to the server log viewer.
    GoToLogViewer,
    /// Navigate to the settings screen.
    GoToSettings,
    /// Request the controller to start a bot actor.
    #[cfg(feature = "cli")]
    StartBot(BotKind),
    /// Request the controller to stop a bot actor.
    #[cfg(feature = "cli")]
    StopBot(BotKind),
    /// Request the controller to restart a bot actor.
    #[cfg(feature = "cli")]
    RestartBot(BotKind),
    /// Serialize `content` and write it to `path`.
    SaveNarrative {
        /// Destination path.
        path: PathBuf,
        /// Serialized TOML string (produced by the editor screen).
        toml: String,
    },
    /// Exit the TUI cleanly.
    Quit,
}

/// Which bot actor to target for start/stop/restart commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
pub enum BotKind {
    /// Content generation bot.
    #[display("generation_bot")]
    Generation,
    /// Content curation bot.
    #[display("curation_bot")]
    Curation,
    /// Content posting bot.
    #[display("posting_bot")]
    Posting,
}

/// Trait implemented by every screen in the operator console.
///
/// Screens are **pure**: `to_tui_node` takes `&self` and has no side effects.
/// All I/O and shared-state mutations are delegated to
/// [`BotController`](crate::controller::BotController) via `BotTransition`.
pub trait BotScreen: Send {
    /// Produce the ratatui widget tree for the current screen state.
    fn to_tui_node(&self) -> TuiNode;

    /// Handle a key event and return the resulting transition.
    fn handle_key(&mut self, key: KeyEvent, ctx: &BotScreenContext) -> BotTransition;

    /// Human-readable name for this screen (used in status bar).
    fn screen_name(&self) -> &'static str;

    /// Notify the screen that a bot's run state changed.
    ///
    /// Called by [`BotController`](crate::controller::BotController) after
    /// executing a `StartBot` / `StopBot` / `RestartBot` transition. The
    /// default implementation is a no-op; [`BotStatusScreen`] overrides it.
    fn on_bot_state_changed(&mut self, _kind: BotKind, _running: bool) {}
}
