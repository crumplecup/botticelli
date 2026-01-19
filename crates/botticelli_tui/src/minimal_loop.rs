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

    // Spawn background task for MCP HTTP client
    #[cfg(feature = "cli")]
    {
        info!("🚀 Setting up MCP HTTP client");

        // Get MCP server configuration from environment or use defaults
        let mcp_host = std::env::var("MCP_HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let mcp_port: u16 = std::env::var("MCP_HTTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let mcp_url = format!("http://{}:{}/sse", mcp_host, mcp_port);
        info!("📍 MCP server endpoint: {}", mcp_url);

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        // Verify MCP server is reachable, start it if not
        info!("🔍 Verifying MCP server is reachable at {}", mcp_url);
        let test_client = http_client.clone();
        let test_url = mcp_url.clone();

        tokio::spawn(async move {
            // Give server a moment if it's starting up
            tokio::time::sleep(Duration::from_millis(100)).await;

            match test_client.get(&test_url).send().await {
                Ok(response) => {
                    info!(
                        "✅ MCP server already running (status: {})",
                        response.status()
                    );
                }
                Err(e) => {
                    warn!("⚠️  MCP server not reachable: {}", e);
                    info!("🚀 Auto-starting MCP server...");

                    // Start MCP server in background
                    #[cfg(feature = "http")]
                    {
                        use botticelli_mcp::run_pmcp_http_server;

                        tokio::spawn(async move {
                            info!("🌐 Spawning embedded MCP HTTP server");
                            match run_pmcp_http_server(
                                "127.0.0.1",
                                8080,
                                #[cfg(feature = "database")]
                                None,
                            )
                            .await
                            {
                                Ok(_) => info!("✅ Embedded MCP server completed"),
                                Err(e) => warn!("❌ Embedded MCP server failed: {}", e),
                            }
                        });

                        // Wait for server to start
                        info!("⏳ Waiting for MCP server to start...");
                        tokio::time::sleep(Duration::from_millis(1000)).await;

                        // Verify it started
                        match test_client.get(&test_url).send().await {
                            Ok(response) => {
                                info!(
                                    "✅ MCP server auto-started successfully (status: {})",
                                    response.status()
                                );
                            }
                            Err(e) => {
                                warn!("❌ MCP server failed to start: {}", e);
                                warn!("💡 You can start it manually:");
                                warn!(
                                    "   cargo run --bin botticelli-mcp-pmcp-http --features http"
                                );
                            }
                        }
                    }

                    #[cfg(not(feature = "http"))]
                    {
                        warn!("❌ Cannot auto-start server: streamable-http feature not enabled");
                        warn!("💡 Start server manually:");
                        warn!("   cargo run --bin botticelli-mcp-pmcp-http --features http");
                        warn!("💡 Or enable streamable-http feature in botticelli_tui");
                    }
                }
            }
        });

        tokio::spawn(async move {
            info!("🌐 MCP client task started");
            info!("📡 Listening for UI messages to forward to MCP server");

            // Process messages from UI
            while let Some(msg) = ui_rx.recv().await {
                debug!(?msg, "Received UI message");

                match msg {
                    UiMessage::SendChat(text) => {
                        info!(text = %text, "Sending chat to MCP server at {}", mcp_url);

                        // Build MCP tool call request for sampling/createMessage
                        let mcp_request = serde_json::json!({
                            "method": "tools/call",
                            "params": {
                                "name": "sampling_createMessage",
                                "arguments": {
                                    "messages": [{
                                        "role": "user",
                                        "content": text
                                    }],
                                    "max_tokens": 1024
                                }
                            }
                        });

                        // Send HTTP POST request to MCP server /sse endpoint
                        match http_client.post(&mcp_url).json(&mcp_request).send().await {
                            Ok(response) => {
                                let status = response.status();
                                info!(status = ?status, "Got MCP response");

                                match response.text().await {
                                    Ok(body) => {
                                        info!(body_len = body.len(), "MCP response body received");
                                        debug!(body = %body, "Full response body");

                                        if let Err(e) =
                                            bg_tx.send(BackgroundMessage::ChatResponse(body)).await
                                        {
                                            warn!(error = ?e, "Failed to send response to UI");
                                        }
                                    }
                                    Err(e) => {
                                        warn!(error = ?e, "Failed to read MCP response body");
                                        if let Err(e) = bg_tx
                                            .send(BackgroundMessage::Error(e.to_string()))
                                            .await
                                        {
                                            warn!(error = ?e, "Failed to send error to UI");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(error = ?e, "Failed to send to MCP server at {}", mcp_url);
                                warn!(
                                    "💡 Is the MCP server running? Check: cargo run --bin botticelli-mcp-pmcp-http --features http"
                                );

                                if let Err(e) = bg_tx
                                    .send(BackgroundMessage::Error(format!(
                                        "MCP server error: {}",
                                        e
                                    )))
                                    .await
                                {
                                    warn!(error = ?e, "Failed to send error to UI");
                                }
                            }
                        }
                    }
                }
            }

            info!("🛑 MCP client task ending");
        });
    }

    #[cfg(not(feature = "cli"))]
    {
        info!("CLI feature not enabled - no MCP HTTP client");
        info!("💡 To enable MCP client: cargo run --features cli");

        tokio::spawn(async move {
            while let Some(msg) = ui_rx.recv().await {
                if let UiMessage::SendChat(text) = msg {
                    debug!(text = %text, "Mock: would send to MCP server");
                    let _ = bg_tx
                        .send(BackgroundMessage::ChatResponse(format!("Echo: {}", text)))
                        .await;
                }
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
