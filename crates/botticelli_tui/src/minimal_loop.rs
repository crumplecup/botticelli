use crate::{
    AppState, ChatView, TuiResult, View, ViewMode,
    view::{
        BotsView, ConversationHistoryView, DatabaseView, NarrativeBrowserView, NarrativeEditorView,
        ScheduleView, SettingsView,
    },
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
    #[cfg(feature = "cli")]
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
    let (bg_tx, mut bg_rx) = mpsc::channel::<BackgroundMessage>(100);

    // Spawn background task for MCP client
    #[cfg(feature = "cli")]
    {
        let mcp_host = std::env::var("MCP_HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let mcp_port: u16 = std::env::var("MCP_HTTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);
        let mcp_url = format!("http://{}:{}/mcp", mcp_host, mcp_port);
        info!(url = %mcp_url, "Connecting to Botticelli MCP server");

        tokio::spawn(async move {
            let client = match botticelli_mcp_client::BotticelliClient::connect_http(&mcp_url).await
            {
                Ok(c) => {
                    info!("Connected to Botticelli MCP server");
                    c
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        "Could not connect to MCP server — start it with: \
                         cargo run --bin botticelli-mcp -- http"
                    );
                    if let Err(send_err) = bg_tx
                        .send(BackgroundMessage::Error(format!(
                            "MCP server unreachable ({}). \
                             Run: cargo run --bin botticelli-mcp -- http",
                            e
                        )))
                        .await
                    {
                        warn!(error = ?send_err, "Failed to send connection error to UI");
                    }
                    return;
                }
            };

            while let Some(msg) = ui_rx.recv().await {
                debug!(?msg, "Received UI message");
                match msg {
                    UiMessage::SendChat(text) => {
                        info!(text = %text, "Calling generate tool");
                        let mut args = serde_json::Map::new();
                        args.insert("prompt".into(), serde_json::Value::String(text));
                        match client.call_tool("generate", Some(args)).await {
                            Ok(result) => {
                                let response = result
                                    .content
                                    .iter()
                                    .filter_map(|c| c.raw.as_text().map(|t| t.text.as_str()))
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                if let Err(e) =
                                    bg_tx.send(BackgroundMessage::ChatResponse(response)).await
                                {
                                    warn!(error = ?e, "Failed to send response to UI");
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "Tool call failed");
                                if let Err(e) =
                                    bg_tx.send(BackgroundMessage::Error(e.to_string())).await
                                {
                                    warn!(error = ?e, "Failed to send error to UI");
                                }
                            }
                        }
                    }
                }
            }

            info!("MCP client task ending");
        });
    }

    #[cfg(not(feature = "cli"))]
    {
        info!("CLI feature not enabled - no MCP HTTP client");
        info!("💡 To enable MCP client: cargo run --features cli");

        tokio::spawn(async move {
            while let Some(UiMessage::SendChat(text)) = ui_rx.recv().await {
                debug!(text = %text, "Mock: would send to MCP server");
                let _ = bg_tx
                    .send(BackgroundMessage::ChatResponse(format!("Echo: {}", text)))
                    .await;
            }
        });
    }

    info!("Entering event loop");
    loop {
        tokio::select! {
            // Handle background responses
            Some(bg_msg) = bg_rx.recv() => {
                match bg_msg {
                    BackgroundMessage::ChatResponse(response) => {
                        debug!(response = %response, "Received chat response");
                        state.add_chat_response(response);
                    }
                    #[cfg(feature = "cli")]
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
                                // Add user message to conversation FIRST
                                state.add_user_message(msg.clone(), None);
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
