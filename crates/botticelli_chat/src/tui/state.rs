// Application state management

use crate::tui::tabs::{BotsTab, ChatTab, DatabaseTab, NarrativesTab, ScheduleTab, SettingsTab};

/// Main application state
#[derive(Debug)]
pub struct AppState {
    /// Currently active tab
    pub active_tab: Tab,

    /// Tab-specific state
    pub narratives_state: NarrativesTab,
    pub bots_state: BotsTab,
    pub database_state: DatabaseTab,
    pub chat_state: ChatTab,
    pub schedule_state: ScheduleTab,
    pub settings_state: SettingsTab,

    /// Global status
    pub status: StatusInfo,

    /// Active modal
    pub modal: Option<Modal>,

    /// Whether the app should quit
    pub should_quit: bool,
}

/// Available tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Narratives,
    Bots,
    Database,
    Chat,
    Schedule,
    Settings,
}

/// Status information displayed in status bar
#[derive(Debug, Clone)]
pub struct StatusInfo {
    pub current_narrative: Option<String>,
    pub active_provider: String,
    pub db_connected: bool,
    pub mcp_connected: bool,
}

/// Modal dialog variants
#[derive(Debug, Clone)]
pub enum Modal {
    Help,
    Error {
        title: String,
        message: String,
    },
    Confirm {
        message: String,
        on_confirm: ConfirmAction,
    },
    Search {
        query: String,
    },
}

/// Actions that can be confirmed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    DeleteNarrative,
    StopBot,
    ClearDatabase,
}

impl AppState {
    /// Creates a new application state
    pub fn new() -> Self {
        Self {
            active_tab: Tab::Narratives,
            narratives_state: NarrativesTab::new(),
            bots_state: BotsTab::new(),
            database_state: DatabaseTab::new(),
            chat_state: ChatTab::new(),
            schedule_state: ScheduleTab::new(),
            settings_state: SettingsTab::new(),
            status: StatusInfo {
                current_narrative: None,
                active_provider: "Claude".to_string(),
                db_connected: false,
                mcp_connected: false,
            },
            modal: None,
            should_quit: false,
        }
    }

    /// Switches to the next tab
    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Narratives => Tab::Bots,
            Tab::Bots => Tab::Database,
            Tab::Database => Tab::Chat,
            Tab::Chat => Tab::Schedule,
            Tab::Schedule => Tab::Settings,
            Tab::Settings => Tab::Narratives,
        };
    }

    /// Switches to the previous tab
    pub fn previous_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Narratives => Tab::Settings,
            Tab::Bots => Tab::Narratives,
            Tab::Database => Tab::Bots,
            Tab::Chat => Tab::Database,
            Tab::Schedule => Tab::Chat,
            Tab::Settings => Tab::Schedule,
        };
    }

    /// Gets the name of the current tab
    pub fn tab_name(&self) -> &'static str {
        match self.active_tab {
            Tab::Narratives => "Narratives",
            Tab::Bots => "Bots",
            Tab::Database => "Database",
            Tab::Chat => "Chat",
            Tab::Schedule => "Schedule",
            Tab::Settings => "Settings",
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
