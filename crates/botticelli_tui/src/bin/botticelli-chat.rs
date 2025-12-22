//! Botticelli TUI Binary - Minimal keyboard-only version
//!
//! Terminal user interface for Botticelli.

use botticelli_tui::{AppState, minimal_event_loop, TuiResult};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[tokio::main]
async fn main() -> TuiResult<()> {
    // Initialize tracing
    use tracing_subscriber::{fmt, EnvFilter};
    let file = std::fs::File::create("botticelli-chat.log").expect("Failed to create log file");
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("botticelli_tui=debug".parse().unwrap()))
        .with_writer(file)
        .with_ansi(false)
        .init();
    
    tracing::info!("Botticelli TUI starting");
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut state = AppState::default();

    // Run minimal event loop - JUST keyboard input
    let result = minimal_event_loop(&mut terminal, &mut state).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
