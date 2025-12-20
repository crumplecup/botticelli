// Command system for TUI actions

use std::path::PathBuf;

use crate::tui::state::{Modal, Tab};

/// Commands that can be executed in the TUI
#[derive(Debug, Clone)]
pub enum TuiCommand {
    /// Navigation commands
    SwitchTab(Tab),
    NextTab,
    PreviousTab,
    SelectNext,
    SelectPrevious,
    GoBack,

    /// Modal commands
    OpenModal(Modal),

    /// Search
    StartSearch,

    /// Application control
    Quit,
}

/// Actions that can be performed on entities
#[derive(Debug, Clone)]
pub enum Action {
    /// Narrative actions
    CreateNarrative,
    EditNarrative(PathBuf),
    ExecuteNarrative(PathBuf),
    ValidateNarrative(PathBuf),
    DeleteNarrative(PathBuf),

    /// Bot actions
    CreateBot(String),
    AssignNarrative {
        bot_id: String,
        narrative: PathBuf,
    },
    StartBot(String),
    StopBot(String),
    ViewBotLogs(String),

    /// Database actions
    RunQuery(String),
    ExportData {
        table: String,
        format: ExportFormat,
    },
    RefreshData,

    /// Schedule actions
    SchedulePost {
        bot_id: String,
        time: chrono::DateTime<chrono::Utc>,
    },
    CancelSchedule(String),
}


