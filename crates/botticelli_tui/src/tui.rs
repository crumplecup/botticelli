//! Main TUI entry point and coordinator.

use crate::{AppState, Event, McpMessage, TuiResult};
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyModifiers};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, sync::Arc, time::Duration};
use tokio::sync::mpsc;

/// Main TUI coordinator.
///
/// Manages the terminal, event handling, and rendering loop.
pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    state: AppState,
    /// Receiver for MCP execution updates from async tasks
    mcp_rx: mpsc::UnboundedReceiver<McpMessage>,
    /// Receiver for crossterm events
    event_rx: mpsc::UnboundedReceiver<Event>,
}

impl Tui {
    /// Create a new TUI instance.
    pub fn new() -> TuiResult<Self> {
        Self::with_llm(None)
    }

    /// Create a new TUI instance with an LLM backend.
    ///
    /// The chat_host should implement the ChatHost trait, integrating LLM and MCP tools.
    pub fn with_llm(chat_host: Option<Arc<std::sync::Mutex<dyn botticelli_interface::ChatHost>>>) -> TuiResult<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;

        // Create channel for MCP updates
        let (mcp_tx, mcp_rx) = mpsc::unbounded_channel();
        
        // Create channel for crossterm events
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        // Spawn async event reader using crossterm's EventStream
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            
            loop {
                match reader.next().await {
                    Some(Ok(crossterm_event)) => {
                        let tui_event = match crossterm_event {
                            CrosstermEvent::Key(key) => {
                                // Check for quit
                                if key.code == KeyCode::Char('q')
                                    || (key.code == KeyCode::Char('c')
                                        && key.modifiers.contains(KeyModifiers::CONTROL))
                                {
                                    Some(Event::Quit)
                                } else {
                                    Some(Event::Key(key))
                                }
                            }
                            CrosstermEvent::Mouse(mouse) => Some(Event::Mouse(mouse)),
                            CrosstermEvent::Resize(w, h) => Some(Event::Resize(w, h)),
                            _ => None,
                        };
                        
                        if let Some(event) = tui_event {
                            if event_tx.send(event).is_err() {
                                break; // Channel closed, exit task
                            }
                        }
                    }
                    Some(Err(_)) => {
                        // Error reading event, continue
                    }
                    None => {
                        // Stream ended, exit task
                        break;
                    }
                }
            }
        });

        let state = if let Some(host) = chat_host {
            let mut state = AppState::new(host);
            state.with_mcp_channel(Some(mcp_tx));
            state
        } else {
            let mut state = AppState::default();
            state.with_mcp_channel(Some(mcp_tx));
            state
        };

        Ok(Self {
            terminal,
            state,
            mcp_rx,
            event_rx,
        })
    }

    /// Run the TUI event loop.
    pub async fn run(&mut self) -> TuiResult<()> {
        // Setup terminal
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;

        self.terminal.clear()?;

        // Create ticker for periodic renders (60fps = ~16ms)
        let mut ticker = tokio::time::interval(Duration::from_millis(16));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Main event loop with biased select (keyboard events first)
        loop {
            tokio::select! {
                biased;
                
                // Priority 1: Keyboard events (instant)
                Some(event) = self.event_rx.recv() => {
                    let start = std::time::Instant::now();
                    let should_continue = self.handle_event(event).await?;
                    let elapsed = start.elapsed();
                    if elapsed.as_millis() > 10 {
                        tracing::warn!("Event handling took {:?} (SLOW!)", elapsed);
                    }
                    if !should_continue {
                        break;
                    }
                }
                
                // Priority 2: MCP updates
                Some(msg) = self.mcp_rx.recv() => {
                    let event = match msg {
                        McpMessage::Update(update) => Event::McpUpdate(update),
                        McpMessage::Error(error) => Event::McpError(error),
                    };
                    self.handle_event(event).await?;
                }
                
                // Priority 3: Periodic render tick (60fps)
                _ = ticker.tick() => {
                    let start = std::time::Instant::now();
                    self.render()?;
                    let elapsed = start.elapsed();
                    if elapsed.as_millis() > 16 {
                        tracing::warn!("Render took {:?} (dropped frame!)", elapsed);
                    }
                }
            }
        }

        // Cleanup terminal
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
        self.terminal.draw(|frame| {
            use crate::view::View;
            use crate::state::ViewMode;
            
            let result = match state.mode() {
                ViewMode::Chat => crate::view::ChatView.render(frame, state),
                ViewMode::NarrativeBrowser => crate::view::NarrativeBrowserView.render(frame, state),
                ViewMode::ConversationHistory => crate::view::ConversationHistoryView.render(frame, state),
                ViewMode::NarrativeEditor => crate::view::NarrativeEditorView.render(frame, state),
                ViewMode::Settings => crate::view::SettingsView.render(frame, state),
                ViewMode::Bots => crate::view::BotsView.render(frame, state),
                ViewMode::Database => crate::view::DatabaseView.render(frame, state),
                ViewMode::Schedule => crate::view::ScheduleView.render(frame, state),
            };
            
            if let Err(e) = result {
                tracing::error!("Render error: {}", e);
            }
        })?;
        Ok(())
    }

    /// Handle an event.
    ///
    /// Returns `false` if the application should quit.
    async fn handle_event(&mut self, event: Event) -> TuiResult<bool> {
        match event {
            Event::Quit => return Ok(false),
            Event::Key(key_event) => {
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
}
