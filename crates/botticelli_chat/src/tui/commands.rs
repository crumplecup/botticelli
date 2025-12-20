// Command system for TUI actions

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

