//! TUI application coordinator.
//!
//! Wires together views, commands, state, and provides clean library entry points.

use crate::{AppState, Command, Event, EventHandler, McpMessage, TuiResult, View, ViewMode, ChatView};
use crossterm::event::KeyEvent;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, sync::{Arc, Mutex}};
use tokio::sync::mpsc;
use tracing::debug;

/// Main TUI application.
///
/// Coordinates views, commands, state, and event handling.
/// This is the primary library interface - binaries should use this.
pub struct TuiApp {
    /// Terminal backend.
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    /// Event handler.
    events: EventHandler,
    /// Application state.
    state: AppState,
    /// MCP update channel receiver.
    mcp_rx: mpsc::UnboundedReceiver<McpMessage>,
}

impl TuiApp {
    /// Create a new TUI application with ChatHost implementation.
    ///
    /// This is the main entry point for library usage.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_chat::ChatHostImpl;
    /// use botticelli_tui::TuiApp;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let chat_host = ChatHostImpl::new().await?;
    ///
    /// let mut app = TuiApp::new(Arc::new(Mutex::new(chat_host)))?;
    /// app.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(chat_host))]
    pub fn new(chat_host: Arc<Mutex<dyn botticelli_interface::ChatHost>>) -> TuiResult<Self> {
        tracing::debug!("Creating TuiApp");
        
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let events = EventHandler::new(std::time::Duration::from_millis(250));

        // Create channel for MCP updates
        let (mcp_tx, mcp_rx) = mpsc::unbounded_channel();

        // Initialize AppState with ChatHost
        let mut state = AppState::new(chat_host);
        state.with_mcp_channel(Some(mcp_tx));

        tracing::debug!("TuiApp created successfully");

        Ok(Self {
            terminal,
            events,
            state,
            mcp_rx,
        })
    }

    /// Run the TUI application.
    ///
    /// This enters the main event loop and blocks until the user quits.
    pub async fn run(&mut self) -> TuiResult<()> {
        // Setup terminal
        self.setup_terminal()?;

        // Initial render
        self.render()?;

        // Create tick interval for state updates
        let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(250));
        
        // Create render interval (60fps = ~16ms)
        let mut render_interval = tokio::time::interval(std::time::Duration::from_millis(16));
        render_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        
        let mut needs_render = false;

        // Main event loop
        loop {
            tokio::select! {
                // Use biased to prioritize keyboard input over ticks
                biased;
                
                // Handle keyboard/terminal events (HIGHEST PRIORITY - checked first)
                event = Self::read_crossterm_event() => {
                    if let Some(evt) = event? {
                        if !self.handle_event(evt).await? {
                            break;
                        }
                        needs_render = true;
                    }
                }
                
                // Handle MCP updates (HIGH PRIORITY)
                Some(msg) = self.mcp_rx.recv() => {
                    let event = match msg {
                        McpMessage::Update(update) => Event::McpUpdate(update),
                        McpMessage::Error(error) => Event::McpError(error),
                    };
                    if !self.handle_event(event).await? {
                        break;
                    }
                    needs_render = true;
                }
                
                // Render timer (60fps - only if needed)
                _ = render_interval.tick() => {
                    if needs_render {
                        self.render()?;
                        needs_render = false;
                    }
                }
                
                // State update tick (lowest priority)
                _ = tick_interval.tick() => {
                    self.state.update()?;
                    needs_render = true;
                }
            }
        }

        // Cleanup terminal
        self.cleanup_terminal()?;

        Ok(())
    }

    /// Read a crossterm event without blocking.
    async fn read_crossterm_event() -> TuiResult<Option<Event>> {
        tokio::task::spawn_blocking(|| {
            if crossterm::event::poll(std::time::Duration::from_millis(0))? {
                match crossterm::event::read()? {
                    crossterm::event::Event::Key(key) => {
                        if key.code == crossterm::event::KeyCode::Char('q')
                            || (key.code == crossterm::event::KeyCode::Char('c')
                                && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL))
                        {
                            Ok(Some(Event::Quit))
                        } else {
                            Ok(Some(Event::Key(key)))
                        }
                    }
                    crossterm::event::Event::Mouse(mouse) => Ok(Some(Event::Mouse(mouse))),
                    crossterm::event::Event::Resize(w, h) => Ok(Some(Event::Resize(w, h))),
                    _ => Ok(None),
                }
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| crate::TuiError::new(crate::TuiErrorKind::EventRead(format!("Join error: {}", e))))?
    }

    /// Setup terminal for TUI.
    fn setup_terminal(&mut self) -> TuiResult<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;

        self.terminal.clear()?;

        Ok(())
    }

    /// Cleanup terminal after TUI.
    fn cleanup_terminal(&mut self) -> TuiResult<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;

        Ok(())
    }

    /// Render the current view.
    fn render(&mut self) -> TuiResult<()> {
        let state = &self.state;
        let view = ChatView; // For now, always use ChatView
        
        self.terminal.draw(|frame| {
            // Render view
            view.render(frame, state).ok();
        })?;
        Ok(())
    }

    /// Handle an event and return false if should quit.
    async fn handle_event(&mut self, event: Event) -> TuiResult<bool> {
        match event {
            Event::Quit => return Ok(false),
            Event::Key(key_event) => {
                // Check for global keybindings first
                if let Some(command) = self.handle_global_keys(key_event) {
                    return self.handle_command(command).await;
                }
                // Pass to state for view-specific handling
                self.state.handle_key(key_event)?;
            }
            Event::Mouse(mouse_event) => {
                self.state.handle_mouse(mouse_event)?;
            }
            Event::Resize(width, height) => {
                self.state.handle_resize(width, height)?;
            }
            Event::Tick => {
                self.state.update()?;
            }
            Event::McpUpdate(update) => {
                self.state.handle_mcp_update(update)?;
            }
            Event::McpError(error) => {
                self.state.handle_mcp_error(error)?;
            }
        }
        Ok(true)
    }

    /// Handle global keybindings (Tab for view switching, etc.).
    fn handle_global_keys(&self, key: KeyEvent) -> Option<Command> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            // Tab cycles through views
            (KeyCode::Tab, KeyModifiers::NONE) => {
                let next_mode = match self.state.mode() {
                    ViewMode::Chat => ViewMode::ConversationHistory,
                    ViewMode::ConversationHistory => ViewMode::NarrativeBrowser,
                    ViewMode::NarrativeBrowser => ViewMode::NarrativeEditor,
                    ViewMode::NarrativeEditor => ViewMode::Settings,
                    ViewMode::Settings => ViewMode::Chat,
                };
                Some(Command::SwitchMode(next_mode))
            }
            // Shift+Tab cycles backwards
            (KeyCode::BackTab, _) => {
                let prev_mode = match self.state.mode() {
                    ViewMode::Chat => ViewMode::Settings,
                    ViewMode::Settings => ViewMode::NarrativeEditor,
                    ViewMode::NarrativeEditor => ViewMode::NarrativeBrowser,
                    ViewMode::NarrativeBrowser => ViewMode::ConversationHistory,
                    ViewMode::ConversationHistory => ViewMode::Chat,
                };
                Some(Command::SwitchMode(prev_mode))
            }
            // Ctrl+Q always quits
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => Some(Command::Quit),
            _ => None,
        }
    }

    /// Handle a command and return false if should quit.
    async fn handle_command(&mut self, command: Command) -> TuiResult<bool> {
        debug!(?command, "Handling command");

        match command {
            Command::Quit => return Ok(false),
            Command::SwitchMode(mode) => {
                debug!(?mode, "Switching to view mode");
                self.state.with_mode(mode);
            }
            Command::SendMessage(message) => {
                // Send message with orchestration (tool calling support)
                self.state.send_message_with_orchestration(message)?;
            }
            Command::ClearConversation => {
                debug!("Clearing conversation");
                self.state.clear_conversation();
            }
            Command::NavigateUp => match self.state.mode() {
                ViewMode::ConversationHistory => {
                    self.state.select_previous_conversation_history();
                }
                ViewMode::NarrativeBrowser => {
                    if let Some(idx) = self.state.selected_narrative()
                        && *idx > 0
                    {
                        self.state.with_selected_narrative(Some(idx - 1));
                    }
                }
                _ => {}
            },
            Command::NavigateDown => match self.state.mode() {
                ViewMode::ConversationHistory => {
                    self.state.select_next_conversation_history();
                }
                ViewMode::NarrativeBrowser => {
                    if let Some(idx) = self.state.selected_narrative() {
                        let max = self.state.narrative_list().len().saturating_sub(1);
                        if *idx < max {
                            self.state.with_selected_narrative(Some(idx + 1));
                        }
                    } else if !self.state.narrative_list().is_empty() {
                        self.state.with_selected_narrative(Some(0));
                    }
                }
                _ => {}
            },
            Command::AppendChar(c) => {
                self.state.append_input(&c.to_string());
            }
            Command::DeleteChar => {
                self.state.delete_char();
            }
            Command::SelectNarrative => {
                // Handle differently based on current view mode
                match self.state.mode() {
                    ViewMode::ConversationHistory => {
                        // Load selected conversation
                        if let Some(idx) = self.state.selected_conversation_history() {
                            let conversation_ids = self.state.conversation_ids();
                            if let Some(conversation_id) = conversation_ids.get(*idx) {
                                debug!(conversation_id = %conversation_id, "Loading conversation");
                                self.state.with_current_conversation(Some(*conversation_id));
                                self.state.with_mode(ViewMode::Chat);
                            }
                        }
                    }
                    ViewMode::NarrativeBrowser => {
                        // Load selected narrative (not yet implemented)
                        debug!("Load narrative not yet implemented");
                    }
                    _ => {}
                }
            }
            Command::LoadConversation(conversation_id) => {
                debug!(conversation_id = %conversation_id, "Loading conversation");
                self.state.with_current_conversation(Some(conversation_id));
                self.state.with_mode(ViewMode::Chat);
            }
            _ => {
                // Other commands not yet implemented
                debug!(?command, "Command not yet implemented");
            }
        }

        Ok(true)
    }
}
