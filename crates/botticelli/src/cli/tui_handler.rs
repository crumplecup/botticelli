//! TUI launch command handler.

use botticelli_error::BotticelliResult;

/// Launch the terminal user interface for a table.
#[cfg(all(feature = "tui", feature = "database"))]
pub async fn launch_tui(_table: &str) -> BotticelliResult<()> {
    use botticelli_tui::Tui;

    tracing::info!("Launching TUI");

    let mut tui = Tui::new()?;
    tui.run().await?;

    Ok(())
}

#[cfg(not(all(feature = "tui", feature = "database")))]
pub async fn launch_tui(_table: &str) -> BotticelliResult<()> {
    eprintln!("Error: TUI and database features not enabled. Rebuild with --features tui,database");
    std::process::exit(1);
}
