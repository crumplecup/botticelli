//! TUI launch command handler.

use botticelli::BotticelliResult;

/// Launch the terminal user interface for a table.
#[cfg(all(feature = "tui", feature = "database"))]
pub async fn launch_tui(_table: &str) -> BotticelliResult<()> {
    use botticelli_tui::App;

    tracing::info!("Launching TUI");

    let mut app = App::new();
    app.run()?;

    Ok(())
}

#[cfg(not(all(feature = "tui", feature = "database")))]
pub async fn launch_tui(_table: &str) -> BotticelliResult<()> {
    eprintln!("Error: TUI and database features not enabled. Rebuild with --features tui,database");
    std::process::exit(1);
}
