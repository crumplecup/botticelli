//! Main TUI entry point and coordinator.

use crate::{AppState, Event, McpMessage, TuiResult};
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyModifiers};
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
    pub fn with_llm(chat_host: Option<Arc<tokio::sync::Mutex<dyn botticelli_interface::ChatHost>>>) -> TuiResult<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;

        // Create channel for MCP updates
        let (mcp_tx, mcp_rx) = mpsc::unbounded_channel();
        
        // Create channel for crossterm events
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        // Spawn dedicated thread (not tokio blocking pool) for event reading
        // This ensures the event reader isn't starved by other blocking tasks
        std::thread::Builder::new()
            .name("crossterm-events".to_string())
            .spawn(move || {
            tracing::info!("Event reader thread started");
            let mut event_count = 0u64;
            
            loop {
                tracing::trace!("Polling for events...");
                let wait_start = std::time::Instant::now();
                
                // Poll with minimal timeout for instant keyboard response
                // Using 1ms keeps CPU usage reasonable while ensuring responsiveness
                match event::poll(Duration::from_millis(1)) {
                    Ok(true) => {
                        match event::read() {
                            Ok(crossterm_event) => {
                                event_count += 1;
                                let wait_elapsed = wait_start.elapsed();
                                let process_start = std::time::Instant::now();
                                
                                tracing::info!(
                                    event_num = event_count,
                                    wait_time_us = wait_elapsed.as_micros(),
                                    "Got event: {:?}", 
                                    crossterm_event
                                );
                                
                                let tui_event = match crossterm_event {
                                    CrosstermEvent::Key(key) => {
                                        tracing::info!("KEY EVENT: {:?}", key);
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
                                    let send_start = std::time::Instant::now();
                                    if event_tx.send(event).is_err() {
                                        tracing::warn!("Event channel closed");
                                        break;
                                    }
                                    let send_elapsed = send_start.elapsed();
                                    tracing::debug!("Channel send took {:?}", send_elapsed);
                                    if send_elapsed.as_micros() > 100 {
                                        tracing::warn!("Channel send SLOW: {:?}", send_elapsed);
                                    }
                                }
                                
                                let process_elapsed = process_start.elapsed();
                                if process_elapsed.as_micros() > 500 {
                                    tracing::warn!("Event processing SLOW: {:?}", process_elapsed);
                                } else {
                                    tracing::debug!("Event processed in {:?}", process_elapsed);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Event read error: {:?}", e);
                            }
                        }
                    }
                    Ok(false) => {
                        // No event available within timeout, continue polling
                        // This is normal and expected
                    }
                    Err(e) => {
                        tracing::error!("Event poll error: {:?}", e);
                    }
                }
            }
        })?;

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
                
                // Priority 1: Keyboard events (update state ONLY, no render)
                Some(event) = self.event_rx.recv() => {
                    let recv_time = std::time::Instant::now();
                    tracing::debug!("Received event from channel: {:?}", event);
                    
                    let should_continue = self.handle_event(event).await?;
                    
                    let handle_elapsed = recv_time.elapsed();
                    if handle_elapsed.as_millis() > 5 {
                        tracing::warn!("Event handling took {:?} (SLOW!)", handle_elapsed);
                    } else {
                        tracing::debug!("Event handled in {:?}", handle_elapsed);
                    }
                    
                    // NO RENDER HERE - state is updated, dirty flag set
                    // Rendering happens in ticker branch at 60fps
                    
                    if !should_continue {
                        break;
                    }
                }
                
                // Priority 2: MCP updates (update state, no render)
                Some(msg) = self.mcp_rx.recv() => {
                    let event = match msg {
                        McpMessage::Update(update) => Event::McpUpdate(update),
                        McpMessage::Error(error) => Event::McpError(error),
                    };
                    self.handle_event(event).await?;
                    // NO RENDER HERE - handled by ticker
                }
                
                // Priority 3: Periodic render (ONLY place that calls terminal.draw!)
                _ = ticker.tick() => {
                    if self.state.needs_render() {
                        let render_start = std::time::Instant::now();
                        tracing::debug!("Rendering (state is dirty)");
                        
                        self.render()?;
                        self.state.clear_dirty();
                        
                        let render_elapsed = render_start.elapsed();
                        if render_elapsed.as_millis() > 16 {
                            tracing::warn!("Render took {:?} (should be <16ms)", render_elapsed);
                        } else {
                            tracing::debug!("Render completed in {:?}", render_elapsed);
                        }
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
