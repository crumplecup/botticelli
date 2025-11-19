//! Application state and main TUI entry point.

use crate::{Event, EventHandler, TuiError, TuiErrorKind};
use crate::ui;
use botticelli_error::{BotticelliError, BotticelliResult};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use diesel::PgConnection;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

/// Application mode determines which view is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppMode {
    /// List view - browse content items
    List,
    /// Detail view - view single content item
    Detail,
    /// Edit view - edit tags, rating, status
    Edit,
    /// Compare view - side-by-side comparison
    Compare,
    /// Export view - export options
    Export,
}

/// Content row representation for TUI display.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentRow {
    /// Row ID
    pub id: i64,
    /// Review status (pending, approved, rejected)
    pub review_status: String,
    /// User rating (1-5)
    pub rating: Option<i32>,
    /// Tags
    pub tags: Vec<String>,
    /// Content preview (first 50 chars)
    pub preview: String,
    /// Full content (for detail view)
    pub content: serde_json::Value,
    /// Source narrative
    pub source_narrative: Option<String>,
    /// Source act
    pub source_act: Option<String>,
}

/// Edit buffer for inline editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditBuffer {
    /// Tags being edited
    pub tags: String,
    /// Rating being edited (1-5)
    pub rating: Option<i32>,
    /// Status being edited
    pub status: String,
    /// Which field is currently focused
    pub focused_field: EditField,
}

/// Edit field focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EditField {
    /// Tags field
    Tags,
    /// Rating field
    Rating,
    /// Status field
    Status,
}

/// Main application state.
pub struct App {
    /// Current mode
    pub mode: AppMode,
    /// Table name being viewed
    pub table_name: String,
    /// List of content items
    pub content_items: Vec<ContentRow>,
    /// Currently selected index in list
    pub selected_index: usize,
    /// Items selected for comparison
    pub compare_selection: Vec<usize>,
    /// Edit buffer (when in Edit mode)
    pub edit_buffer: Option<EditBuffer>,
    /// Status message to display
    pub status_message: String,
    /// Whether to quit the application
    pub should_quit: bool,
    /// Database connection
    conn: PgConnection,
}

impl App {
    /// Create a new App instance.
    #[tracing::instrument(skip(conn))]
    pub fn new(table_name: String, conn: PgConnection) -> BotticelliResult<Self> {
        let mut app = Self {
            mode: AppMode::List,
            table_name,
            content_items: Vec::new(),
            selected_index: 0,
            compare_selection: Vec::new(),
            edit_buffer: None,
            status_message: String::from("Press ? for help"),
            should_quit: false,
            conn,
        };

        // Load initial content
        app.reload_content()?;

        Ok(app)
    }

    /// Reload content from database.
    #[tracing::instrument(skip(self))]
    pub fn reload_content(&mut self) -> BotticelliResult<()> {
        use botticelli_database::list_content;

        let items = list_content(&mut self.conn, &self.table_name, None, 1000)?;

        self.content_items = items
            .into_iter()
            .filter_map(|item| {
                let id = item.get("id").and_then(|v| v.as_i64())?;
                let review_status = item
                    .get("review_status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let rating = item
                    .get("rating")
                    .and_then(|v| v.as_i64())
                    .map(|r| r as i32);
                let tags = item
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|t| t.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                // Create a preview from the JSON content
                let preview = serde_json::to_string(&item)
                    .unwrap_or_default()
                    .chars()
                    .take(50)
                    .collect::<String>();

                let source_narrative = item
                    .get("source_narrative")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let source_act = item
                    .get("source_act")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                Some(ContentRow {
                    id,
                    review_status,
                    rating,
                    tags,
                    preview,
                    content: item,
                    source_narrative,
                    source_act,
                })
            })
            .collect();

        if self.selected_index >= self.content_items.len() && !self.content_items.is_empty() {
            self.selected_index = self.content_items.len() - 1;
        }

        Ok(())
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if !self.content_items.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if self.selected_index < self.content_items.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Enter detail view for selected item.
    pub fn enter_detail(&mut self) {
        if !self.content_items.is_empty() {
            self.mode = AppMode::Detail;
        }
    }

    /// Return to list view.
    pub fn return_to_list(&mut self) {
        self.mode = AppMode::List;
        self.edit_buffer = None;
        self.compare_selection.clear();
    }

    /// Enter edit mode for selected item.
    #[tracing::instrument(skip(self))]
    pub fn enter_edit(&mut self) -> BotticelliResult<()> {
        if let Some(item) = self.content_items.get(self.selected_index) {
            self.edit_buffer = Some(EditBuffer {
                tags: item.tags.join(", "),
                rating: item.rating,
                status: item.review_status.clone(),
                focused_field: EditField::Tags,
            });
            self.mode = AppMode::Edit;
        }
        Ok(())
    }

    /// Save edits to database.
    #[tracing::instrument(skip(self))]
    pub fn save_edit(&mut self) -> BotticelliResult<()> {
        if let Some(buffer) = &self.edit_buffer {
            let item_id = self.content_items[self.selected_index].id;
            let tags: Vec<String> = buffer
                .tags
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            use botticelli_database::{update_content_metadata, update_review_status};

            update_content_metadata(
                &mut self.conn,
                &self.table_name,
                item_id,
                Some(&tags),
                buffer.rating,
            )?;

            update_review_status(&mut self.conn, &self.table_name, item_id, &buffer.status)?;

            self.reload_content()?;
            self.status_message = format!("Saved changes to item {}", item_id);
            self.return_to_list();
        }
        Ok(())
    }

    /// Toggle item in comparison selection.
    pub fn toggle_compare(&mut self) {
        if let Some(pos) = self
            .compare_selection
            .iter()
            .position(|&i| i == self.selected_index)
        {
            self.compare_selection.remove(pos);
        } else {
            self.compare_selection.push(self.selected_index);
        }

        if self.compare_selection.len() >= 2 {
            self.mode = AppMode::Compare;
        }
    }

    /// Delete selected item.
    #[tracing::instrument(skip(self))]
    pub fn delete_selected(&mut self) -> BotticelliResult<()> {
        if let Some(item) = self.content_items.get(self.selected_index) {
            use botticelli_database::delete_content;

            let item_id = item.id;
            delete_content(&mut self.conn, &self.table_name, item_id)?;
            self.reload_content()?;
            self.status_message = format!("Deleted item {}", item_id);
        }
        Ok(())
    }

    /// Promote selected item to target table.
    #[tracing::instrument(skip(self))]
    pub fn promote_selected(&mut self, target: &str) -> BotticelliResult<()> {
        if let Some(item) = self.content_items.get(self.selected_index) {
            use botticelli_database::promote_content;

            let new_id = promote_content(&mut self.conn, &self.table_name, target, item.id)?;
            self.status_message = format!("Promoted to {} with ID {}", target, new_id);
        }
        Ok(())
    }

    /// Quit the application.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

/// Run the TUI application.
#[tracing::instrument(skip(conn))]
pub fn run_tui(table_name: String, conn: PgConnection) -> BotticelliResult<()> {
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
    let mut app = App::new(table_name, conn)?;
    let mut events = EventHandler::new(250);

    // Main event loop
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

    terminal.show_cursor().map_err(|e| {
        BotticelliError::from(TuiError::new(TuiErrorKind::TerminalRestore(format!(
            "show cursor: {}",
            e
        ))))
    })?;

    result
}

/// Run the application event loop.
#[tracing::instrument(skip_all)]
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &mut EventHandler,
) -> BotticelliResult<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, app)).map_err(|e| {
            BotticelliError::from(TuiError::new(TuiErrorKind::Rendering(e.to_string())))
        })?;

        if let Some(event) = events.next()? {
            match event {
                Event::Tick => {}
                Event::Key(key) => handle_key_event(app, key)?,
            }
        }
    }

    Ok(())
}

/// Handle keyboard input.
#[tracing::instrument(skip(app))]
fn handle_key_event(app: &mut App, key: crossterm::event::KeyEvent) -> BotticelliResult<()> {
    use crossterm::event::{KeyCode, KeyModifiers};

    match app.mode {
        AppMode::List => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => app.quit(),
            KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Enter => app.enter_detail(),
            KeyCode::Char('e') => app.enter_edit()?,
            KeyCode::Char('c') => app.toggle_compare(),
            KeyCode::Char('d') => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.delete_selected()?;
                }
            }
            KeyCode::Char('r') => app.reload_content()?,
            _ => {}
        },
        AppMode::Detail => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.return_to_list(),
            KeyCode::Char('e') => app.enter_edit()?,
            _ => {}
        },
        AppMode::Edit => match key.code {
            KeyCode::Esc => app.return_to_list(),
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.save_edit()?;
            }
            _ => {
                // Handle text input in edit mode (TODO: implement in next iteration)
            }
        },
        AppMode::Compare => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.return_to_list(),
            _ => {}
        },
        AppMode::Export => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.return_to_list(),
            _ => {}
        },
    }

    Ok(())
}
