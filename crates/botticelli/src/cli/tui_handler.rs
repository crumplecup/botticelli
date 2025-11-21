//! TUI launch command handler.

use botticelli::BotticelliResult;

/// Launch the terminal user interface for a table.
#[cfg(all(feature = "tui", feature = "database"))]
pub async fn launch_tui(table: &str) -> BotticelliResult<()> {
    use botticelli_tui::{run_tui, DatabaseBackend};

    tracing::info!(table = %table, "Launching TUI");
    
    let mut backend = DatabaseBackend::new()?;
    run_tui(&mut backend, table.to_string())?;

    Ok(())
}

#[cfg(not(all(feature = "tui", feature = "database")))]
pub async fn launch_tui(_table: &str) -> BotticelliResult<()> {
    eprintln!("Error: TUI and database features not enabled. Rebuild with --features tui,database");
    std::process::exit(1);
}
