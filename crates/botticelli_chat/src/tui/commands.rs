//! TUI command system.

use crate::tui::Tab;

/// Commands that can be executed in the TUI.
#[derive(Debug, Clone)]
pub enum TuiCommand {
    /// Navigation commands
    SwitchTab(Tab),
    NextTab,
    PrevTab,
    SelectNext,
    SelectPrevious,

    /// Modal commands
    OpenHelp,
    CloseModal,
    
    /// Application commands
    Quit,
}
