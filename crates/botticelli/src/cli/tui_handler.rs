//! TUI launch command handler.

use botticelli::BotticelliResult;

/// Launch the terminal user interface for a table.
#[cfg(feature = "tui")]
pub async fn launch_tui(table: &str) -> BotticelliResult<()> {
    use botticelli::establish_connection;
    use botticelli_tui::run_tui;

    eprintln!("Launching TUI for table: {}", table);

    let conn = establish_connection()?;
    run_tui(table.to_string(), conn)?;

    Ok(())
}

#[cfg(not(feature = "tui"))]
pub async fn launch_tui(_table: &str) -> BotticelliResult<()> {
    eprintln!("Error: TUI feature not enabled. Rebuild with --features tui");
    std::process::exit(1);
}
