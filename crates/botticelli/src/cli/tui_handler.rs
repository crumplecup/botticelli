//! TUI launch command handler.

use botticelli_error::BotticelliResult;

/// Launch the terminal user interface.
///
/// The TUI is a separate binary (`botticelli-tui`). This stub tells the user
/// how to run it when they invoke the `tui` subcommand on the main binary.
#[tracing::instrument]
pub async fn launch_tui(_table: &str) -> BotticelliResult<()> {
    eprintln!("The TUI is a separate binary. Run: botticelli-tui");
    std::process::exit(1);
}
