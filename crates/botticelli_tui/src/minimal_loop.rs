use crate::{
    AppState, ChatView, TuiResult, View, ViewMode,
    view::{BotsView, ConversationHistoryView, DatabaseView, NarrativeBrowserView,
           NarrativeEditorView, ScheduleView, SettingsView},
};
use crossterm::event::{Event, EventStream, KeyCode};
use futures::StreamExt;
use ratatui::prelude::*;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, instrument, warn};

/// Message from UI to background task
#[derive(Debug, Clone)]
pub enum UiMessage {
    SendChat(String),
}

/// Message from background task to UI
#[derive(Debug, Clone)]
pub enum BackgroundMessage {
    ChatResponse(String),
    Error(String),
}

/// Minimal event loop using tokio::select! for instant keyboard response
#[instrument(skip(terminal, state))]
pub async fn minimal_event_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
) -> TuiResult<()> {
    info!("Starting minimal event loop");
    
    // Set up rendering at 60 FPS
    let fps = 60.0;
    let period = Duration::from_secs_f32(1.0 / fps);
    let mut interval = tokio::time::interval(period);
    info!("Interval set to {}ms", period.as_millis());
    
    // Set up event stream for async keyboard input
    let mut events = EventStream::new();
    info!("Event stream created");
    
    // Set up channels for background communication
    let (ui_tx, mut ui_rx) = mpsc::channel::<UiMessage>(100);
    let (_bg_tx, mut bg_rx) = mpsc::channel::<BackgroundMessage>(100);
    
    // Spawn background task for MCP HTTP client
    #[cfg(feature = "cli")]
    info!("Spawning MCP HTTP client background task");
    #[cfg(not(feature = "cli"))]
    info!("CLI feature not enabled - no MCP HTTP client");
    
    #[cfg(feature = "cli")]
    let http_client = reqwest::Client::new();
    #[cfg(feature = "cli")]
    let mcp_url = "http://localhost:3000".to_string(); // TODO: Get from config
    
    tokio::spawn(async move {
        info!("MCP client task started");
        
        // Process messages from UI
        while let Some(msg) = ui_rx.recv().await {
            debug!(?msg, "Received UI message");
            
            match msg {
                UiMessage::SendChat(text) => {
                    #[cfg(feature = "cli")]
                    {
                        info!(text = %text, "Sending chat to MCP server");
                        
                        // Send HTTP request to MCP server
                        match http_client
                            .post(&format!("{}/chat", mcp_url))
                            .json(&serde_json::json!({ "message": text }))
                            .send()
                            .await
                        {
                            Ok(response) => {
                                info!(status = ?response.status(), "Got MCP response");
                                match response.text().await {
                                    Ok(body) => {
                                        info!(body = %body, "MCP response body");
                                        // TODO: Parse and send to UI via bg_tx
                                    }
                                    Err(e) => {
                                        warn!(error = ?e, "Failed to read MCP response body");
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(error = ?e, "Failed to send to MCP server");
                            }
                        }
                    }
                    
                    #[cfg(not(feature = "cli"))]
                    debug!(text = %text, "Mock: would send to MCP server");
                }
            }
        }
        
        info!("MCP client task ending");
    });
    
    info!("Entering event loop");
    loop {
        tokio::select! {
            // Handle background responses
            Some(bg_msg) = bg_rx.recv() => {
                match bg_msg {
                    BackgroundMessage::ChatResponse(response) => {
                        debug!(response = %response, "Received chat response");
                        // TODO: Add to conversation
                    }
                    BackgroundMessage::Error(err) => {
                        warn!(error = %err, "Background error");
                    }
                }
            }
            
            // Render on tick
            _ = interval.tick() => {
                terminal.draw(|f| {
                    // Add view mode indicator to frame
                    use ratatui::layout::{Constraint, Direction, Layout};
                    use ratatui::widgets::{Paragraph};
                    
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(1), Constraint::Min(1)])
                        .split(f.area());
                    
                    // Top status bar showing current view
                    let status = format!("View: {:?} | Tab: Switch | Ctrl+C: Quit", state.mode());
                    f.render_widget(
                        Paragraph::new(status)
                            .style(ratatui::style::Style::default()
                                .fg(ratatui::style::Color::Yellow)),
                        chunks[0],
                    );
                    
                    // Dynamically render view based on mode
                    match state.mode() {
                        ViewMode::Chat => {
                            if let Err(e) = ChatView.render(f, state) {
                                debug!(?e, "Chat render error");
                            }
                        }
                        ViewMode::ConversationHistory => {
                            if let Err(e) = ConversationHistoryView.render(f, state) {
                                debug!(?e, "ConversationHistory render error");
                            }
                        }
                        ViewMode::NarrativeBrowser => {
                            if let Err(e) = NarrativeBrowserView.render(f, state) {
                                debug!(?e, "NarrativeBrowser render error");
                            }
                        }
                        ViewMode::NarrativeEditor => {
                            if let Err(e) = NarrativeEditorView.render(f, state) {
                                debug!(?e, "NarrativeEditor render error");
                            }
                        }
                        ViewMode::Settings => {
                            if let Err(e) = SettingsView.render(f, state) {
                                debug!(?e, "Settings render error");
                            }
                        }
                        ViewMode::Bots => {
                            if let Err(e) = BotsView.render(f, state) {
                                debug!(?e, "Bots render error");
                            }
                        }
                        ViewMode::Database => {
                            if let Err(e) = DatabaseView.render(f, state) {
                                debug!(?e, "Database render error");
                            }
                        }
                        ViewMode::Schedule => {
                            if let Err(e) = ScheduleView.render(f, state) {
                                debug!(?e, "Schedule render error");
                            }
                        }
                    }
                })?;
            }
            
            // Handle keyboard events immediately
            Some(Ok(event)) = events.next() => {
                if let Event::Key(key) = event {
                    debug!(?key, modifiers = ?key.modifiers, "Key event received");
                    
                    // Handle quit
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        info!("Ctrl+C received - quitting");
                        return Ok(());
                    }
                    
                    // Handle Tab for view switching
                    if key.code == KeyCode::Tab {
                        let old_mode = *state.mode();
                        debug!(?old_mode, "Tab pressed - switching view");
                        let new_mode = old_mode.next();
                        debug!(?new_mode, "Calculated next view");
                        state.with_mode(new_mode);
                        let current = *state.mode();
                        info!(old = ?old_mode, new = ?new_mode, current = ?current, "View switched");
                        continue;
                    }
                    
                    // Handle BackTab for previous view
                    if key.code == KeyCode::BackTab {
                        let old_mode = *state.mode();
                        debug!(?old_mode, "BackTab pressed - switching to previous view");
                        let new_mode = old_mode.previous();
                        debug!(?new_mode, "Calculated previous view");
                        state.with_mode(new_mode);
                        let current = *state.mode();
                        info!(old = ?old_mode, new = ?new_mode, current = ?current, "View switched (previous)");
                        continue;
                    }

                    // Use current view's input handler
                    let command_result = match state.mode() {
                        ViewMode::Chat => ChatView.handle_input(key, state),
                        ViewMode::ConversationHistory => ConversationHistoryView.handle_input(key, state),
                        ViewMode::NarrativeBrowser => NarrativeBrowserView.handle_input(key, state),
                        ViewMode::NarrativeEditor => NarrativeEditorView.handle_input(key, state),
                        ViewMode::Settings => SettingsView.handle_input(key, state),
                        ViewMode::Bots => BotsView.handle_input(key, state),
                        ViewMode::Database => DatabaseView.handle_input(key, state),
                        ViewMode::Schedule => ScheduleView.handle_input(key, state),
                    };
                    
                    if let Some(command) = command_result? {
                        debug!(?command, "Executing command");
                        
                        // Execute the command
                        match command {
                            crate::Command::AppendChar(c) => {
                                state.append_input(&c.to_string());
                            }
                            crate::Command::DeleteChar => {
                                state.delete_char();
                            }
                            crate::Command::SendMessage(msg) => {
                                debug!(message = %msg, "Sending message");
                                state.clear_input();
                                // Send to background task (non-blocking)
                                if let Err(e) = ui_tx.send(UiMessage::SendChat(msg)).await {
                                    warn!(error = ?e, "Failed to send to background task");
                                }
                            }
                            crate::Command::ClearConversation => {
                                state.clear_input();
                            }
                            crate::Command::Quit => {
                                return Ok(());
                            }
                            _ => {
                                debug!(?command, "Unhandled command");
                            }
                        }
                    }
                }
            }
        }
    }
}
