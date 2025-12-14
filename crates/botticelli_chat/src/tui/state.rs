//! Application state management.

use crate::ServiceContainer;
use std::sync::Arc;

/// Main application state.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Currently active tab
    pub active_tab: Tab,

    /// Tab-specific state
    pub narratives_state: NarrativesTabState,
    pub bots_state: BotsTabState,
    pub database_state: DatabaseTabState,
    pub chat_state: ChatTabState,
    pub schedule_state: ScheduleTabState,
    pub settings_state: SettingsTabState,

    /// Global state
    pub status: StatusInfo,
    pub modal: Option<Modal>,
}

impl AppState {
    /// Create new application state.
    pub fn new() -> Self {
        Self {
            active_tab: Tab::Chat,
            narratives_state: NarrativesTabState::default(),
            bots_state: BotsTabState::default(),
            database_state: DatabaseTabState::default(),
            chat_state: ChatTabState::default(),
            schedule_state: ScheduleTabState::default(),
            settings_state: SettingsTabState::default(),
            status: StatusInfo::default(),
            modal: None,
        }
    }

    /// Switch to the next tab.
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

    /// Switch to the previous tab.
    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Narratives => Tab::Settings,
            Tab::Bots => Tab::Narratives,
            Tab::Database => Tab::Bots,
            Tab::Chat => Tab::Database,
            Tab::Schedule => Tab::Chat,
            Tab::Settings => Tab::Schedule,
        };
    }

    /// Switch to a specific tab.
    pub fn switch_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Available tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Narrative browser and manager
    Narratives,
    /// Bot manager
    Bots,
    /// Database browser
    Database,
    /// Chat interface
    Chat,
    /// Scheduling
    Schedule,
    /// Settings
    Settings,
}

impl Tab {
    /// Get tab title for display.
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Narratives => "Narratives",
            Tab::Bots => "Bots",
            Tab::Database => "Database",
            Tab::Chat => "Chat",
            Tab::Schedule => "Schedule",
            Tab::Settings => "Settings",
        }
    }

    /// Get tab index (for keyboard shortcuts).
    pub fn index(&self) -> usize {
        match self {
            Tab::Narratives => 1,
            Tab::Bots => 2,
            Tab::Database => 3,
            Tab::Chat => 4,
            Tab::Schedule => 5,
            Tab::Settings => 6,
        }
    }

    /// Get tab from index.
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            1 => Some(Tab::Narratives),
            2 => Some(Tab::Bots),
            3 => Some(Tab::Database),
            4 => Some(Tab::Chat),
            5 => Some(Tab::Schedule),
            6 => Some(Tab::Settings),
            _ => None,
        }
    }
}

/// Status information displayed in status bar.
#[derive(Debug, Clone, Default)]
pub struct StatusInfo {
    /// Current narrative (if any)
    pub current_narrative: Option<String>,
    /// Active LLM provider
    pub active_provider: String,
    /// Database connection status
    pub db_connected: bool,
    /// MCP connection status
    pub mcp_connected: bool,
}

/// Modal dialog overlay.
#[derive(Debug, Clone)]
pub enum Modal {
    /// Help overlay
    Help,
    /// Error message
    Error { message: String },
    /// Confirmation dialog
    Confirm {
        message: String,
        action: ConfirmAction,
    },
    /// Search/filter interface
    Search {
        query: String,
        results: Vec<String>,
    },
}

/// Action to take on confirmation.
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    /// Delete a narrative
    DeleteNarrative(String),
    /// Stop a bot
    StopBot(String),
    /// Quit application
    Quit,
}

// Tab-specific state structures

/// Narratives tab state.
#[derive(Debug, Clone, Default)]
pub struct NarrativesTabState {
    /// Selected narrative index
    pub selected_index: usize,
    /// Filter query
    pub filter: Option<String>,
}

/// Bots tab state.
#[derive(Debug, Clone, Default)]
pub struct BotsTabState {
    /// Selected bot index
    pub selected_index: usize,
}

/// Database tab state.
#[derive(Debug, Clone, Default)]
pub struct DatabaseTabState {
    /// Selected table index
    pub selected_index: usize,
}

/// Chat tab state.
#[derive(Debug, Clone, Default)]
pub struct ChatTabState {
    /// Messages in current conversation
    pub messages: Vec<ChatMessage>,
    /// Scroll offset
    pub scroll_offset: usize,
}

/// Chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Message role
    pub role: MessageRole,
    /// Message content
    pub content: String,
}

/// Message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Schedule tab state.
#[derive(Debug, Clone, Default)]
pub struct ScheduleTabState {
    /// Selected schedule index
    pub selected_index: usize,
}

/// Settings tab state.
#[derive(Debug, Clone, Default)]
pub struct SettingsTabState {
    /// Selected setting index
    pub selected_index: usize,
}
