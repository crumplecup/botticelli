// Event handling system

#[cfg(feature = "tui")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::tui::{AppState, TuiCommand};

/// Result of event handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// Event was handled
    Handled,
    /// Event was not handled
    NotHandled,
    /// Application should quit
    ShouldQuit,
}

/// Event handler for processing input events
pub struct EventHandler;

impl EventHandler {
    /// Creates a new event handler
    pub fn new() -> Self {
        Self
    }

    #[cfg(feature = "tui")]
    /// Processes an event and returns the result
    pub fn handle_event(&self, event: Event, state: &mut AppState) -> EventResult {
        // Handle modal events first
        if state.modal.is_some() {
            return self.handle_modal_event(event, state);
        }

        // Handle global keyboard shortcuts
        if let Event::Key(key) = event {
            // First try global commands
            if let Some(command) = self.key_to_command(key, state) {
                return self.execute_command(command, state);
            }

            // Then try tab-specific actions
            if let Some(result) = self.handle_tab_specific(key, state) {
                return result;
            }
        }

        EventResult::NotHandled
    }

    #[cfg(feature = "tui")]
    /// Handles tab-specific keyboard input
    fn handle_tab_specific(&self, key: KeyEvent, state: &mut AppState) -> Option<EventResult> {
        use crate::tui::state::Tab;

        match state.active_tab {
            Tab::Narratives => {
                // Handle narrative navigation
                state.narratives_state.handle_nav_key(key.code);

                // Handle narrative actions
                if let KeyCode::Char(c) = key.code {
                    if let Some(action) = state.narratives_state.handle_action_key(c) {
                        return Some(self.execute_narrative_action(action, state));
                    }
                }

                Some(EventResult::Handled)
            }
            Tab::Chat => {
                // Chat tab handles its own input
                if state.chat_state.handle_key(key) {
                    Some(EventResult::Handled)
                } else {
                    None
                }
            }
            Tab::Bots => {
                // Bots tab handles its own input
                if state.bots_state.handle_key(key.code) {
                    Some(EventResult::Handled)
                } else {
                    None
                }
            }
            Tab::Database => {
                // Database tab handles its own input
                if state.database_state.handle_key(key.code) {
                    Some(EventResult::Handled)
                } else {
                    None
                }
            }
            Tab::Schedule => {
                // Schedule tab handles its own input
                if state.schedule_state.handle_key(key.code) {
                    Some(EventResult::Handled)
                } else {
                    None
                }
            }
            Tab::Settings => {
                // Settings tab handles its own input
                if state.settings_state.handle_key(key.code) {
                    Some(EventResult::Handled)
                } else {
                    None
                }
            }
        }
    }

    #[cfg(feature = "tui")]
    /// Executes a narrative action
    fn execute_narrative_action(
        &self,
        action: crate::tui::tabs::narratives::NarrativeAction,
        state: &mut AppState,
    ) -> EventResult {
        use crate::tui::tabs::narratives::NarrativeAction;

        match action {
            NarrativeAction::Edit(path) => {
                // TODO: Launch external editor
                state.modal = Some(crate::tui::state::Modal::Error {
                    title: "Not Implemented".to_string(),
                    message: format!("Edit action for {} not yet implemented", path.display()),
                });
                EventResult::Handled
            }
            NarrativeAction::Execute(path) => {
                // TODO: Execute narrative
                state.modal = Some(crate::tui::state::Modal::Error {
                    title: "Not Implemented".to_string(),
                    message: format!("Execute action for {} not yet implemented", path.display()),
                });
                EventResult::Handled
            }
            NarrativeAction::Validate(path) => {
                // Validate by attempting to load
                match botticelli_narrative::Narrative::from_file(&path) {
                    Ok(_) => {
                        state.modal = Some(crate::tui::state::Modal::Error {
                            title: "Validation Success".to_string(),
                            message: format!("✓ {} is valid", path.display()),
                        });
                    }
                    Err(e) => {
                        state.modal = Some(crate::tui::state::Modal::Error {
                            title: "Validation Error".to_string(),
                            message: format!("✗ {}\n\nError: {}", path.display(), e),
                        });
                    }
                }
                EventResult::Handled
            }
            NarrativeAction::Delete(_path) => {
                // TODO: Confirm and delete
                state.modal = Some(crate::tui::state::Modal::Error {
                    title: "Not Implemented".to_string(),
                    message: "Delete action not yet implemented".to_string(),
                });
                EventResult::Handled
            }
            NarrativeAction::CreateNew => {
                // TODO: Launch creation wizard
                state.modal = Some(crate::tui::state::Modal::Error {
                    title: "Not Implemented".to_string(),
                    message: "Create new narrative not yet implemented".to_string(),
                });
                EventResult::Handled
            }
            NarrativeAction::Refresh => {
                // TODO: Refresh narratives from disk
                state.modal = Some(crate::tui::state::Modal::Error {
                    title: "Refresh".to_string(),
                    message: "Refreshing narratives... (not yet implemented)".to_string(),
                });
                EventResult::Handled
            }
        }
    }

    #[cfg(feature = "tui")]
    /// Handles events when a modal is open
    fn handle_modal_event(&self, event: Event, state: &mut AppState) -> EventResult {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    state.modal = None;
                    return EventResult::Handled;
                }
                KeyCode::Enter => {
                    // Handle modal confirmation
                    state.modal = None;
                    return EventResult::Handled;
                }
                _ => {}
            }
        }
        EventResult::Handled
    }

    #[cfg(feature = "tui")]
    /// Converts a key event to a command
    fn key_to_command(&self, key: KeyEvent, _state: &AppState) -> Option<TuiCommand> {
        match (key.code, key.modifiers) {
            // Quit
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Char('q'), KeyModifiers::NONE) => Some(TuiCommand::Quit),

            // Tab navigation
            (KeyCode::Tab, KeyModifiers::NONE) => Some(TuiCommand::NextTab),
            (KeyCode::BackTab, KeyModifiers::SHIFT) => Some(TuiCommand::PreviousTab),

            // Number keys for direct tab access
            (KeyCode::Char('1'), KeyModifiers::NONE) => {
                Some(TuiCommand::SwitchTab(crate::tui::state::Tab::Narratives))
            }
            (KeyCode::Char('2'), KeyModifiers::NONE) => {
                Some(TuiCommand::SwitchTab(crate::tui::state::Tab::Bots))
            }
            (KeyCode::Char('3'), KeyModifiers::NONE) => {
                Some(TuiCommand::SwitchTab(crate::tui::state::Tab::Database))
            }
            (KeyCode::Char('4'), KeyModifiers::NONE) => {
                Some(TuiCommand::SwitchTab(crate::tui::state::Tab::Chat))
            }
            (KeyCode::Char('5'), KeyModifiers::NONE) => {
                Some(TuiCommand::SwitchTab(crate::tui::state::Tab::Schedule))
            }
            (KeyCode::Char('6'), KeyModifiers::NONE) => {
                Some(TuiCommand::SwitchTab(crate::tui::state::Tab::Settings))
            }

            // Help
            (KeyCode::Char('?'), KeyModifiers::NONE) => {
                Some(TuiCommand::OpenModal(crate::tui::state::Modal::Help))
            }

            // Search
            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(TuiCommand::StartSearch),

            // Navigation
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                Some(TuiCommand::SelectNext)
            }
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                Some(TuiCommand::SelectPrevious)
            }

            // Escape
            (KeyCode::Esc, KeyModifiers::NONE) => Some(TuiCommand::GoBack),

            _ => None,
        }
    }

    /// Executes a command and updates state
    fn execute_command(&self, command: TuiCommand, state: &mut AppState) -> EventResult {
        match command {
            TuiCommand::Quit => {
                state.should_quit = true;
                EventResult::ShouldQuit
            }
            TuiCommand::NextTab => {
                state.next_tab();
                EventResult::Handled
            }
            TuiCommand::PreviousTab => {
                state.previous_tab();
                EventResult::Handled
            }
            TuiCommand::SwitchTab(tab) => {
                state.active_tab = tab;
                EventResult::Handled
            }
            TuiCommand::OpenModal(modal) => {
                state.modal = Some(modal);
                EventResult::Handled
            }
            _ => {
                // Other commands not yet implemented
                EventResult::NotHandled
            }
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
