// Database tab implementation

use serde_json::Value as JsonValue;

#[cfg(feature = "tui")]
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

/// Database view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Browsing list of tables
    Tables,
    /// Viewing table schema
    Schema,
    /// Browsing table content
    Content,
}

/// Table information
#[derive(Debug, Clone)]
pub struct TableInfo {
    /// Table name
    pub name: String,
    /// Row count (if known)
    pub row_count: Option<i64>,
    /// Whether this is a generated content table
    pub is_content_table: bool,
}

impl TableInfo {
    /// Creates a new table info
    pub fn new(name: String) -> Self {
        let is_content_table = name.starts_with("content_") ||
                              name.starts_with("generated_") ||
                              !["narratives", "narrative_executions", "act_executions",
                                "act_inputs", "model_responses", "actor_server_state",
                                "actor_server_executions"].contains(&name.as_str());

        Self {
            name,
            row_count: None,
            is_content_table,
        }
    }

    /// Sets the row count
    pub fn with_row_count(mut self, count: i64) -> Self {
        self.row_count = Some(count);
        self
    }
}

/// Column information for schema display
#[derive(Debug, Clone)]
pub struct ColumnDisplay {
    /// Column name
    pub name: String,
    /// Data type
    pub data_type: String,
    /// Whether nullable
    pub nullable: bool,
    /// Default value
    pub default: Option<String>,
}

/// Content row for display
#[derive(Debug, Clone)]
pub struct ContentRow {
    /// Row data as JSON
    pub data: JsonValue,
    /// Selected for detailed view
    pub selected: bool,
}

/// Filter options for content browsing
#[derive(Debug, Clone)]
pub struct ContentFilter {
    /// Review status filter ("pending", "approved", "rejected", or None for all)
    pub review_status: Option<String>,
    /// Maximum number of rows to display
    pub limit: usize,
}

impl Default for ContentFilter {
    fn default() -> Self {
        Self {
            review_status: None,
            limit: 100,
        }
    }
}

/// State for the Database tab
#[derive(Debug)]
pub struct DatabaseTab {
    /// Current view mode
    view_mode: ViewMode,

    /// List of available tables
    tables: Vec<TableInfo>,

    /// Currently selected table
    selected_table_index: usize,

    /// Schema for selected table
    schema: Vec<ColumnDisplay>,

    /// Content rows for selected table
    content: Vec<ContentRow>,

    /// Content filter settings
    filter: ContentFilter,

    /// Connection status
    connected: bool,

    /// Error message if any
    error_message: Option<String>,

    /// List state for table selection
    #[cfg(feature = "tui")]
    table_list_state: ListState,

    /// List state for content browsing
    #[cfg(feature = "tui")]
    content_list_state: ListState,

    /// Scroll offset for schema view
    schema_scroll: usize,

    /// Scroll offset for content detail view
    detail_scroll: usize,
}

impl DatabaseTab {
    /// Creates a new database tab
    pub fn new() -> Self {
        // Create some placeholder system tables
        let tables = vec![
            TableInfo::new("narratives".to_string()),
            TableInfo::new("narrative_executions".to_string()),
            TableInfo::new("act_executions".to_string()),
            TableInfo::new("model_responses".to_string()),
        ];

        Self {
            view_mode: ViewMode::Tables,
            tables,
            selected_table_index: 0,
            schema: Vec::new(),
            content: Vec::new(),
            filter: ContentFilter::default(),
            connected: false,
            error_message: None,
            #[cfg(feature = "tui")]
            table_list_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
            #[cfg(feature = "tui")]
            content_list_state: ListState::default(),
            schema_scroll: 0,
            detail_scroll: 0,
        }
    }

    /// Sets connection status
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    /// Updates the list of tables
    pub fn set_tables(&mut self, tables: Vec<TableInfo>) {
        self.tables = tables;
        if !self.tables.is_empty() && self.selected_table_index >= self.tables.len() {
            self.selected_table_index = 0;
            #[cfg(feature = "tui")]
            {
                self.table_list_state.select(Some(0));
            }
        }
    }

    /// Gets the currently selected table
    pub fn selected_table(&self) -> Option<&TableInfo> {
        self.tables.get(self.selected_table_index)
    }

    /// Sets the schema for the current table
    pub fn set_schema(&mut self, schema: Vec<ColumnDisplay>) {
        self.schema = schema;
        self.schema_scroll = 0;
    }

    /// Sets the content for the current table
    pub fn set_content(&mut self, content: Vec<JsonValue>) {
        self.content = content
            .into_iter()
            .map(|data| ContentRow {
                data,
                selected: false,
            })
            .collect();

        #[cfg(feature = "tui")]
        {
            if !self.content.is_empty() {
                self.content_list_state.select(Some(0));
            } else {
                self.content_list_state.select(None);
            }
        }
        self.detail_scroll = 0;
    }

    /// Sets an error message
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    /// Clears error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Switches to table list view
    pub fn show_tables(&mut self) {
        self.view_mode = ViewMode::Tables;
        self.clear_error();
    }

    /// Switches to schema view for selected table
    pub fn show_schema(&mut self) {
        if self.selected_table().is_some() {
            self.view_mode = ViewMode::Schema;
            self.clear_error();
            // TODO: Trigger schema loading
        }
    }

    /// Switches to content view for selected table
    pub fn show_content(&mut self) {
        if self.selected_table().is_some() {
            self.view_mode = ViewMode::Content;
            self.clear_error();
            // TODO: Trigger content loading
        }
    }

    /// Moves table selection up
    pub fn select_previous_table(&mut self) {
        if self.tables.is_empty() {
            return;
        }

        if self.selected_table_index == 0 {
            self.selected_table_index = self.tables.len() - 1;
        } else {
            self.selected_table_index -= 1;
        }

        #[cfg(feature = "tui")]
        {
            self.table_list_state.select(Some(self.selected_table_index));
        }
    }

    /// Moves table selection down
    pub fn select_next_table(&mut self) {
        if self.tables.is_empty() {
            return;
        }

        if self.selected_table_index >= self.tables.len() - 1 {
            self.selected_table_index = 0;
        } else {
            self.selected_table_index += 1;
        }

        #[cfg(feature = "tui")]
        {
            self.table_list_state.select(Some(self.selected_table_index));
        }
    }

    /// Moves content row selection up
    #[cfg(feature = "tui")]
    pub fn select_previous_row(&mut self) {
        if self.content.is_empty() {
            return;
        }

        let current = self.content_list_state.selected().unwrap_or(0);
        let new_index = if current == 0 {
            self.content.len() - 1
        } else {
            current - 1
        };

        self.content_list_state.select(Some(new_index));
        self.detail_scroll = 0;
    }

    /// Moves content row selection down
    #[cfg(feature = "tui")]
    pub fn select_next_row(&mut self) {
        if self.content.is_empty() {
            return;
        }

        let current = self.content_list_state.selected().unwrap_or(0);
        let new_index = if current >= self.content.len() - 1 {
            0
        } else {
            current + 1
        };

        self.content_list_state.select(Some(new_index));
        self.detail_scroll = 0;
    }

    /// Scrolls schema view up
    pub fn scroll_schema_up(&mut self) {
        if self.schema_scroll > 0 {
            self.schema_scroll -= 1;
        }
    }

    /// Scrolls schema view down
    pub fn scroll_schema_down(&mut self) {
        self.schema_scroll += 1;
    }

    /// Scrolls detail view up
    pub fn scroll_detail_up(&mut self) {
        if self.detail_scroll > 0 {
            self.detail_scroll -= 1;
        }
    }

    /// Scrolls detail view down
    pub fn scroll_detail_down(&mut self) {
        self.detail_scroll += 1;
    }

    /// Toggles review status filter (cycles through None -> pending -> approved -> rejected -> None)
    pub fn cycle_filter(&mut self) {
        self.filter.review_status = match self.filter.review_status.as_deref() {
            None => Some("pending".to_string()),
            Some("pending") => Some("approved".to_string()),
            Some("approved") => Some("rejected".to_string()),
            _ => None,
        };
        // TODO: Trigger content reload
    }

    /// Handles keyboard input
    #[cfg(feature = "tui")]
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;

        match key {
            // Global navigation
            KeyCode::Char('t') => {
                self.show_tables();
                true
            }
            KeyCode::Char('s') => {
                self.show_schema();
                true
            }
            KeyCode::Char('c') => {
                self.show_content();
                true
            }
            KeyCode::Esc => {
                self.show_tables();
                true
            }

            // Mode-specific navigation
            _ => {
                match self.view_mode {
                    ViewMode::Tables => self.handle_table_nav(key),
                    ViewMode::Schema => self.handle_schema_nav(key),
                    ViewMode::Content => self.handle_content_nav(key),
                }
            }
        }
    }

    #[cfg(feature = "tui")]
    /// Handles navigation in table list view
    fn handle_table_nav(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous_table();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next_table();
                true
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.show_content();
                true
            }
            _ => false,
        }
    }

    #[cfg(feature = "tui")]
    /// Handles navigation in schema view
    fn handle_schema_nav(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_schema_up();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_schema_down();
                true
            }
            _ => false,
        }
    }

    #[cfg(feature = "tui")]
    /// Handles navigation in content view
    fn handle_content_nav(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous_row();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next_row();
                true
            }
            KeyCode::PageUp => {
                self.scroll_detail_up();
                true
            }
            KeyCode::PageDown => {
                self.scroll_detail_down();
                true
            }
            KeyCode::Char('f') => {
                self.cycle_filter();
                true
            }
            _ => false,
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the database tab
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        match self.view_mode {
            ViewMode::Tables => self.render_tables(area, buf),
            ViewMode::Schema => self.render_schema(area, buf),
            ViewMode::Content => self.render_content(area, buf),
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the table list view
    fn render_tables(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::{StatefulWidget, Widget};

        if !self.connected {
            let error = Paragraph::new(vec![
                Line::from(""),
                Line::from("Not connected to database"),
                Line::from(""),
                Line::from("Configure database connection in Settings"),
            ])
            .block(Block::default().borders(Borders::ALL).title("Database"))
            .style(Style::default().fg(Color::Red));

            Widget::render(error, area, buf);
            return;
        }

        if let Some(error) = &self.error_message {
            let error_widget = Paragraph::new(vec![
                Line::from(""),
                Line::from("Error:"),
                Line::from(error.as_str()),
            ])
            .block(Block::default().borders(Borders::ALL).title("Database Error"))
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: false });

            Widget::render(error_widget, area, buf);
            return;
        }

        if self.tables.is_empty() {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from("No tables found"),
                Line::from(""),
                Line::from("Database appears to be empty"),
            ])
            .block(Block::default().borders(Borders::ALL).title("Database"))
            .style(Style::default().fg(Color::DarkGray));

            Widget::render(empty, area, buf);
            return;
        }

        // Split into list and help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),      // Table list
                Constraint::Length(3),   // Help
            ])
            .split(area);

        // Render table list
        let items: Vec<ListItem> = self
            .tables
            .iter()
            .map(|table| {
                let icon = if table.is_content_table { "📄" } else { "📊" };
                let count_str = table
                    .row_count
                    .map(|c| format!(" ({} rows)", c))
                    .unwrap_or_default();

                let line = Line::from(vec![
                    Span::raw(format!("{} ", icon)),
                    Span::raw(&table.name),
                    Span::styled(
                        count_str,
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Tables ({} total)", self.tables.len()))
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, chunks[0], buf, &mut self.table_list_state);

        // Render help
        let help = Paragraph::new("↑↓/jk: Navigate | Enter/l: View Content | s: Schema | c: Content | t: Tables");
        Widget::render(help, chunks[1], buf);
    }

    #[cfg(feature = "tui")]
    /// Renders the schema view
    fn render_schema(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;

        let table_name = self
            .selected_table()
            .map(|t| t.name.as_str())
            .unwrap_or("Unknown");

        if self.schema.is_empty() {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from("Loading schema..."),
                Line::from(""),
                Line::from("Schema information will appear here"),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Schema: {}", table_name))
            )
            .style(Style::default().fg(Color::DarkGray));

            Widget::render(empty, area, buf);
            return;
        }

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Table: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(table_name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Columns:", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
        ];

        // Add column information with scrolling
        for (i, col) in self.schema.iter().enumerate().skip(self.schema_scroll) {
            if lines.len() > area.height as usize - 8 {
                break;
            }

            let nullable = if col.nullable { "NULL" } else { "NOT NULL" };
            let default_str = col
                .default
                .as_ref()
                .map(|d| format!(" DEFAULT {}", d))
                .unwrap_or_default();

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", i + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    &col.name,
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(&col.data_type, Style::default().fg(Color::Cyan)),
                Span::raw(format!(" {}{}", nullable, default_str)),
            ]));

            lines.push(Line::from(""));
        }

        let schema_widget = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Schema: {} ({} columns)", table_name, self.schema.len()))
            )
            .wrap(Wrap { trim: false });

        Widget::render(schema_widget, area, buf);
    }

    #[cfg(feature = "tui")]
    /// Renders the content view
    fn render_content(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::{StatefulWidget, Widget};

        let table_name = self
            .selected_table()
            .map(|t| t.name.as_str())
            .unwrap_or("Unknown");

        // Split into content list and detail view
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),  // Content list
                Constraint::Percentage(55),  // Detail view
                Constraint::Length(3),       // Help
            ])
            .split(area);

        // Render content list
        if self.content.is_empty() {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from("No content found"),
                Line::from(""),
                Line::from("Table is empty or loading..."),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Content: {}", table_name))
            )
            .style(Style::default().fg(Color::DarkGray));

            Widget::render(empty, chunks[0], buf);
        } else {
            let items: Vec<ListItem> = self
                .content
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    // Extract a few key fields for the list view
                    let preview = self.extract_row_preview(&row.data);
                    let line = Line::from(vec![
                        Span::styled(
                            format!("{:3} ", i + 1),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw(preview),
                    ]);

                    ListItem::new(line)
                })
                .collect();

            let filter_str = match self.filter.review_status.as_deref() {
                Some(status) => format!(" ({})", status),
                None => " (all)".to_string(),
            };

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Content: {}{} - {} rows", table_name, filter_str, self.content.len()))
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            StatefulWidget::render(list, chunks[0], buf, &mut self.content_list_state);
        }

        // Render detail view for selected row
        if let Some(selected_idx) = self.content_list_state.selected() {
            if let Some(row) = self.content.get(selected_idx) {
                self.render_row_detail(&row.data, chunks[1], buf);
            }
        } else {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from("Select a row to view details"),
            ])
            .block(Block::default().borders(Borders::ALL).title("Row Details"))
            .style(Style::default().fg(Color::DarkGray));

            Widget::render(empty, chunks[1], buf);
        }

        // Render help
        let help = Paragraph::new("↑↓/jk: Navigate | f: Filter | PgUp/PgDn: Scroll Detail | s: Schema | t: Tables");
        Widget::render(help, chunks[2], buf);
    }

    #[cfg(feature = "tui")]
    /// Extracts a preview string from a row for list display
    fn extract_row_preview(&self, data: &JsonValue) -> String {
        // Try to extract meaningful fields in priority order
        if let Some(obj) = data.as_object() {
            // Try to get an ID
            if let Some(id) = obj.get("id") {
                let id_str = match id {
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::String(s) => s.clone(),
                    _ => id.to_string(),
                };

                // Try to get a text/content/title field
                let content = obj
                    .get("content")
                    .or_else(|| obj.get("text"))
                    .or_else(|| obj.get("title"))
                    .or_else(|| obj.get("name"));

                if let Some(content_val) = content {
                    if let Some(text) = content_val.as_str() {
                        let preview = if text.len() > 60 {
                            format!("{}...", &text[..57])
                        } else {
                            text.to_string()
                        };
                        return format!("#{} - {}", id_str, preview);
                    }
                }

                return format!("#{}", id_str);
            }
        }

        "Row".to_string()
    }

    #[cfg(feature = "tui")]
    /// Renders detailed view of a single row
    fn render_row_detail(&self, data: &JsonValue, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;

        let mut lines = vec![Line::from("")];

        if let Some(obj) = data.as_object() {
            let mut sorted_keys: Vec<_> = obj.keys().collect();
            sorted_keys.sort();

            for (i, key) in sorted_keys.iter().enumerate().skip(self.detail_scroll) {
                if lines.len() > area.height as usize - 4 {
                    break;
                }

                if let Some(value) = obj.get(*key) {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("{}: ", key),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    let value_str = match value {
                        JsonValue::String(s) => s.clone(),
                        JsonValue::Number(n) => n.to_string(),
                        JsonValue::Bool(b) => b.to_string(),
                        JsonValue::Null => "null".to_string(),
                        JsonValue::Array(arr) => format!("[{} items]", arr.len()),
                        JsonValue::Object(_) => "{object}".to_string(),
                    };

                    // Wrap long values
                    for line in value_str.lines() {
                        if line.len() > 80 {
                            for chunk in line.as_bytes().chunks(80) {
                                if let Ok(s) = std::str::from_utf8(chunk) {
                                    lines.push(Line::from(format!("  {}", s)));
                                }
                            }
                        } else {
                            lines.push(Line::from(format!("  {}", line)));
                        }
                    }

                    lines.push(Line::from(""));
                }
            }
        } else {
            lines.push(Line::from(format!("{:#?}", data)));
        }

        let detail = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Row Details"))
            .wrap(Wrap { trim: false });

        Widget::render(detail, area, buf);
    }
}

impl Default for DatabaseTab {
    fn default() -> Self {
        Self::new()
    }
}
