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
    /// Send a chat message to the LLM; controller executes and calls back.
    ChatSend {
        /// User input text to send.
        content: String,
    },
    /// Serialize `content` and write it to `path`.
    SaveNarrative {
        /// Destination path.
        path: PathBuf,
        /// Serialized TOML string (produced by the editor screen).
        toml: String,
    },
    /// Fetch rows for a named storage table; controller loads and calls back.
    LoadDatabaseTable {
        /// Logical table name (e.g. `"narrative_executions"`).
        table: String,
    },
    /// Fetch actor state list; controller loads and calls [`BotScreen::on_schedule_loaded`].
    LoadScheduleData,
    /// Fetch execution history for one task; controller loads and calls
    /// [`BotScreen::on_task_executions_loaded`].
    LoadTaskExecutions {
        /// Task identifier to load executions for.
        task_id: String,
    },
    /// Fetch (or refresh) the log file tail; controller loads and calls
    /// [`BotScreen::on_log_lines_loaded`].
    LoadLogLines,
    /// Persist the selected log level to `botticelli-settings.env`.
    SaveSettings {
        /// `RUST_LOG` value to write (e.g. `"debug"`, `"info"`).
        rust_log: String,
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

    /// Deliver fetched storage table rows to the screen.
    ///
    /// Called by the controller after a `LoadDatabaseTable` transition
    /// completes. Each string is a pre-formatted one-line summary of one row.
    /// The default implementation is a no-op; [`DatabaseBrowserScreen`] overrides it.
    fn on_table_loaded(&mut self, _table: &str, _rows: Vec<String>) {}

    /// Deliver fetched actor-state rows and their task IDs to the screen.
    ///
    /// Called after `LoadScheduleData` completes. `task_rows` are pre-formatted
    /// display strings; `task_ids` are the corresponding task identifier strings
    /// (parallel slices). [`ScheduleScreen`] overrides this.
    fn on_schedule_loaded(&mut self, _task_rows: Vec<String>, _task_ids: Vec<String>) {}

    /// Deliver execution history rows for the currently selected task.
    ///
    /// Called after `LoadTaskExecutions` completes. [`ScheduleScreen`] overrides this.
    fn on_task_executions_loaded(&mut self, _exec_rows: Vec<String>) {}

    /// Deliver fresh log file lines to the screen.
    ///
    /// Called after `LoadLogLines` completes. [`LogViewerScreen`] overrides this.
    fn on_log_lines_loaded(&mut self, _lines: Vec<String>) {}
}
