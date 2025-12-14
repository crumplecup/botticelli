use std::io;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing::{debug, instrument};

use crate::{
    AppState, ChatView, Command, Event, EventHandler, NarrativeBrowserView, NarrativeEditorView,
    TuiError, TuiErrorKind, TuiResult, View, ViewMode,
};

/// Main TUI application coordinator.
#[derive(Debug)]
pub struct App {
    /// Application state.
    state: AppState,
    /// Event handler.
    events: EventHandler,
    /// Chat view.
    chat_view: ChatView,
    /// Narrative browser view.
    narrative_browser_view: NarrativeBrowserView,
    /// Narrative editor view.
    narrative_editor_view: NarrativeEditorView,
    /// Whether the app should quit.
    should_quit: bool,
}

impl App {
    /// Creates a new TUI application.
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
            events: EventHandler::default(),
            chat_view: ChatView,
            narrative_browser_view: NarrativeBrowserView,
            narrative_editor_view: NarrativeEditorView,
            should_quit: false,
        }
    }

    /// Runs the TUI application.
    #[instrument(skip(self))]
    pub fn run(&mut self) -> TuiResult<()> {
        debug!("Setting up terminal");
        let mut terminal = self.setup_terminal()?;

        debug!("Starting event loop");
        while !self.should_quit {
            // Render current view
            terminal
                .draw(|frame| {
                    if let Err(e) = self.render(frame) {
                        tracing::error!(error = ?e, "Render failed");
                    }
                })
                .map_err(|e| {
                    TuiError::new(TuiErrorKind::Rendering(format!("Draw failed: {}", e)))
                })?;

            // Handle events
            match self.events.next()? {
                Event::Key(key) => {
                    if let Some(cmd) = self.current_view().handle_input(key, &self.state)? {
                        self.handle_command(cmd)?;
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal will handle resize automatically
                }
                Event::Tick => {
                    // Periodic update - could refresh data here
                }
            }
        }

        debug!("Restoring terminal");
        self.restore_terminal(terminal)?;

        Ok(())
    }

    /// Sets up the terminal for TUI rendering.
    fn setup_terminal(&self) -> TuiResult<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()
            .map_err(|e| TuiError::new(TuiErrorKind::TerminalSetup(format!("{}", e))))?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)
            .map_err(|e| TuiError::new(TuiErrorKind::TerminalSetup(format!("{}", e))))?;

        let backend = CrosstermBackend::new(stdout);
        Terminal::new(backend)
            .map_err(|e| TuiError::new(TuiErrorKind::TerminalSetup(format!("{}", e))))
    }

    /// Restores the terminal to its original state.
    fn restore_terminal(
        &self,
        mut terminal: Terminal<CrosstermBackend<io::Stdout>>,
    ) -> TuiResult<()> {
        disable_raw_mode()
            .map_err(|e| TuiError::new(TuiErrorKind::TerminalRestore(format!("{}", e))))?;

        execute!(terminal.backend_mut(), LeaveAlternateScreen)
            .map_err(|e| TuiError::new(TuiErrorKind::TerminalRestore(format!("{}", e))))?;

        terminal
            .show_cursor()
            .map_err(|e| TuiError::new(TuiErrorKind::TerminalRestore(format!("{}", e))))?;

        Ok(())
    }

    /// Renders the current view.
    fn render(&self, frame: &mut ratatui::Frame) -> TuiResult<()> {
        self.current_view().render(frame, &self.state)
    }

    /// Gets the current view based on mode.
    fn current_view(&self) -> &dyn View {
        match self.state.mode() {
            ViewMode::Chat => &self.chat_view,
            ViewMode::NarrativeBrowser => &self.narrative_browser_view,
            ViewMode::NarrativeEditor => &self.narrative_editor_view,
            ViewMode::Settings => &self.chat_view, // TODO: Settings view
        }
    }

    /// Handles a command.
    #[instrument(skip(self))]
    fn handle_command(&mut self, cmd: Command) -> TuiResult<()> {
        debug!(command = ?cmd, "Handling command");

        match cmd {
            Command::SendMessage(msg) => {
                debug!(message = %msg, "Sending message");
                // TODO: Integrate with conversation session
                self.state.clear_input();
            }
            Command::SwitchMode(mode) => {
                debug!(mode = ?mode, "Switching mode");
                self.state.set_mode(mode);
            }
            Command::NewConversation => {
                debug!("Creating new conversation");
                // TODO: Create conversation
            }
            Command::LoadConversation(id) => {
                debug!(conversation_id = ?id, "Loading conversation");
                self.state.set_current_conversation(Some(id));
            }
            Command::NewNarrative => {
                debug!("Creating new narrative");
                // TODO: Create narrative
            }
            Command::LoadNarrative(id) => {
                debug!(narrative_id = ?id, "Loading narrative");
                self.state.set_current_narrative(Some(id));
            }
            Command::SaveNarrative => {
                debug!("Saving narrative");
                // TODO: Save narrative
            }
            Command::NavigateUp => {
                debug!("Navigating up");
                self.state.select_previous_narrative();
            }
            Command::NavigateDown => {
                debug!("Navigating down");
                self.state.select_next_narrative();
            }
            Command::SelectNarrative => {
                debug!("Selecting narrative");
                if let Some(idx) = self.state.selected_narrative() {
                    if let Some(name) = self.state.narrative_list().get(idx) {
                        debug!(narrative = %name, "Loading narrative for editing");
                        // TODO: Load narrative content
                        self.state.set_editor_content(format!("Content of: {}", name));
                        self.state.set_mode(ViewMode::NarrativeEditor);
                    }
                }
            }
            Command::Quit => {
                debug!("Quitting application");
                self.should_quit = true;
            }
        }

        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
