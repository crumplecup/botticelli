//! botticelli-tui: operator console for the Botticelli bot server.

use botticelli_tui::{BotController, BotScreenContext, TuiError, TuiErrorKind, TuiResult};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> TuiResult<()> {
    // Dual-layer logging: stderr + append log file
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("botticelli-tui.log")
        .map_err(|e| TuiError::new(TuiErrorKind::TerminalSetup(e.to_string())))?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = tracing_subscriber::fmt::layer().with_writer(io::stderr);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(log_file)
        .with_ansi(false);

    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();

    info!("botticelli-tui starting");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Build context (Phase 3+ will wire in BotServer)
    let ctx = BotScreenContext {
        narratives_dir: Some(
            std::env::current_dir()
                .unwrap_or_default()
                .join("crates/botticelli_narrative/narratives/discord"),
        ),
        log_file: Some(std::path::PathBuf::from("botticelli-server.log")),
    };

    let mut controller = BotController::new(ctx);
    let result = controller.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
