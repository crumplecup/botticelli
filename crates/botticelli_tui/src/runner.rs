//! TUI runner - main loop and backend integration.
//!
//! This module contains the main TUI loop that works with any backend
//! implementing the TuiBackend trait.

use crate::{App, Event, EventHandler, TuiBackend, TuiError, TuiErrorKind, TuiResult};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Run the TUI with the provided backend.
///
/// # Arguments
///
/// * `backend` - Backend implementation for data operations
/// * `table_name` - Name of the table to browse
#[cfg(feature = "database")]
pub fn run_tui(backend: &mut dyn TuiBackend, table_name: String) -> TuiResult<()> {
    // Setup terminal
    enable_raw_mode().map_err(|e| TuiError::new(TuiErrorKind::TerminalSetup(format!("Failed to enable raw mode: {}", e))))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| TuiError::new(TuiErrorKind::TerminalSetup(format!("Failed to setup terminal: {}", e))))?;
    
    let backend_impl = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend_impl)
        .map_err(|e| TuiError::new(TuiErrorKind::TerminalSetup(format!("Failed to create terminal: {}", e))))?;

    // Create app state
    let mut app = App::new(table_name.clone());
    let events = EventHandler::new(250);

    // Initial load
    let items = backend.list_content(&table_name, 100)?;
    app.set_content(items);

    // Main loop
    while !app.should_quit {
        terminal
            .draw(|f| crate::ui::draw(f, &app))
            .map_err(|e| TuiError::new(TuiErrorKind::Rendering(format!("Failed to draw: {}", e))))?;

        if let Ok(Some(event)) = events.next() {
            handle_event(&mut app, backend, &table_name, event)?;
        }
    }

    // Cleanup terminal
    disable_raw_mode().map_err(|e| TuiError::new(TuiErrorKind::TerminalRestore(format!("Failed to disable raw mode: {}", e))))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| TuiError::new(TuiErrorKind::TerminalRestore(format!("Failed to cleanup terminal: {}", e))))?;
    terminal
        .show_cursor()
        .map_err(|e| TuiError::new(TuiErrorKind::TerminalRestore(format!("Failed to show cursor: {}", e))))?;

    Ok(())
}

/// Handle a single event.
fn handle_event(
    app: &mut App,
    backend: &mut dyn TuiBackend,
    table_name: &str,
    event: Event,
) -> TuiResult<()> {
    use crossterm::event::{KeyCode, KeyModifiers};

    match event {
        Event::Key(key) => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => app.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
            KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Enter => app.enter_detail(),
            KeyCode::Char('e') => app.enter_edit(),
            KeyCode::Char('c') => app.toggle_compare(),
            KeyCode::Char('d') => {
                if let Some(id) = app.get_selected_id() {
                    backend.delete_item(table_name, id)?;
                    // Reload content
                    let items = backend.list_content(table_name, 100)?;
                    app.set_content(items);
                    app.status_message = "Item deleted".to_string();
                }
            }
            KeyCode::Char('s') if app.mode == crate::AppMode::Edit => {
                if let Some((id, tags, rating, status)) = app.get_edit_data() {
                    backend.update_metadata(table_name, id, &tags, rating, &status)?;
                    // Reload content
                    let items = backend.list_content(table_name, 100)?;
                    app.set_content(items);
                    app.return_to_list();
                    app.status_message = "Changes saved".to_string();
                }
            }
            KeyCode::Backspace if app.mode == crate::AppMode::List => app.return_to_list(),
            _ => {}
        },
        Event::Tick => {}
    }

    Ok(())
}
