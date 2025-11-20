//! TUI launch command handler.

use botticelli::BotticelliResult;

/// Launch the terminal user interface for a table.
#[cfg(feature = "tui")]
pub async fn launch_tui(table: &str) -> BotticelliResult<()> {
    use botticelli::establish_connection;
    use botticelli_tui::run_tui;

    tracing::info!(table = %table, "Launching TUI");

    let conn = establish_connection()?;
    run_tui(table.to_string(), conn)?;

    Ok(())
}

#[cfg(not(feature = "tui"))]
pub async fn launch_tui(_table: &str) -> BotticelliResult<()> {
    eprintln!("Error: TUI feature not enabled. Rebuild with --features tui");
    std::process::exit(1);
}

/// Launch the terminal user interface in server management mode.
#[cfg(all(feature = "tui", feature = "server"))]
pub async fn launch_server_tui() -> BotticelliResult<()> {
    use botticelli::establish_connection;
    use botticelli_tui::{App, AppMode, EventHandler, run_app, ServerView};
    use botticelli_error::BotticelliError;
    use botticelli_tui::{TuiError, TuiErrorKind};
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };
    use ratatui::{Terminal, backend::CrosstermBackend};
    use std::io;

    tracing::info!("Launching TUI in server mode");

    // Setup terminal
    enable_raw_mode().map_err(|e| {
        BotticelliError::from(TuiError::new(TuiErrorKind::TerminalSetup(format!(
            "enable raw mode: {}",
            e
        ))))
    })?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
        BotticelliError::from(TuiError::new(TuiErrorKind::TerminalSetup(format!(
            "alternate screen/mouse capture: {}",
            e
        ))))
    })?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| {
        BotticelliError::from(TuiError::new(TuiErrorKind::TerminalSetup(format!(
            "create terminal: {}",
            e
        ))))
    })?;

    // Create app state
    let conn = establish_connection()?;
    let mut app = App::new(String::new(), conn)?;
    app.mode = AppMode::Server;
    
    // Use default model directory
    let model_dir = dirs::home_dir()
        .ok_or_else(|| BotticelliError::new(botticelli_error::BotticelliErrorKind::Tui(
            TuiError::new(TuiErrorKind::TerminalSetup("Cannot determine home directory".to_string()))
        )))?
        .join(".botticelli/models");
    
    app.server_view = Some(ServerView::new(model_dir));
    
    let mut events = EventHandler::new(250);

    // Run the app
    let result = run_app(&mut terminal, &mut app, &mut events);

    // Restore terminal
    disable_raw_mode().map_err(|e| {
        BotticelliError::from(TuiError::new(TuiErrorKind::TerminalRestore(format!(
            "disable raw mode: {}",
            e
        ))))
    })?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| {
        BotticelliError::from(TuiError::new(TuiErrorKind::TerminalRestore(format!(
            "leave alternate screen: {}",
            e
        ))))
    })?;

    result
}
