//! Botticelli TUI Binary - Minimal keyboard-only version
//!
//! Terminal user interface for Botticelli.

use botticelli_tui::{AppState, TuiResult, minimal_event_loop};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tracing::info;

#[tokio::main]
async fn main() -> TuiResult<()> {
    // Initialize tracing
    use tracing_subscriber::{EnvFilter, fmt};
    let file = std::fs::File::create("botticelli-chat.log").expect("Failed to create log file");
    fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("botticelli_tui=debug".parse().unwrap()),
        )
        .with_writer(file)
        .with_ansi(false)
        .init();

    info!("Botticelli TUI starting");

    // Create HTTP client for MCP server communication
    let mcp_client = reqwest::Client::new();
    let mcp_url = "http://localhost:3030/mcp".to_string();
    info!("MCP client configured for {}", mcp_url);

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
