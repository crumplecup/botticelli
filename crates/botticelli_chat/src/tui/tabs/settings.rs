// Settings tab implementation

#[cfg(feature = "tui")]
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

/// Settings category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCategory {
    /// System information
    System,
    /// Database configuration
    Database,
    /// MCP server configuration
    Mcp,
    /// Observability configuration
    Observability,
    /// Narratives configuration
    Narratives,
    /// UI preferences
    Interface,
}

impl SettingsCategory {
    /// Gets all categories
    pub fn all() -> Vec<Self> {
        vec![
            Self::System,
            Self::Database,
            Self::Mcp,
            Self::Observability,
            Self::Narratives,
            Self::Interface,
        ]
    }

    /// Gets category name
    pub fn name(&self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Database => "Database",
            Self::Mcp => "MCP",
            Self::Observability => "Observability",
            Self::Narratives => "Narratives",
            Self::Interface => "Interface",
        }
    }

    /// Gets category icon
    pub fn icon(&self) -> &'static str {
        match self {
            Self::System => "ℹ",
            Self::Database => "🗄",
            Self::Mcp => "🔌",
            Self::Observability => "📊",
            Self::Narratives => "📖",
            Self::Interface => "🎨",
        }
    }
}

/// A single setting item
#[derive(Debug, Clone)]
pub struct SettingItem {
    /// Setting key/name
    pub key: String,
    /// Setting value
    pub value: String,
    /// Whether this setting is read-only
    pub read_only: bool,
    /// Optional description
    pub description: Option<String>,
}

impl SettingItem {
    /// Creates a new setting item
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            read_only: true,
            description: None,
        }
    }

    /// Adds a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// State for the Settings tab
#[derive(Debug)]
pub struct SettingsTab {
    /// Currently selected category
    selected_category: SettingsCategory,

    /// Settings for each category
    system_settings: Vec<SettingItem>,
    database_settings: Vec<SettingItem>,
    mcp_settings: Vec<SettingItem>,
    observability_settings: Vec<SettingItem>,
    narratives_settings: Vec<SettingItem>,
    interface_settings: Vec<SettingItem>,

    /// List state for category selection
    #[cfg(feature = "tui")]
    category_list_state: ListState,

    /// List state for settings within category
    #[cfg(feature = "tui")]
    setting_list_state: ListState,

    /// Scroll offset for details view
    detail_scroll: usize,
}

impl SettingsTab {
    /// Creates a new settings tab
    pub fn new() -> Self {
        // Initialize with placeholder settings
        let system_settings = vec![
            SettingItem::new("Version", env!("CARGO_PKG_VERSION"))
                .with_description("Botticelli Chat version"),
            SettingItem::new("Rust Version", "1.80+")
                .with_description("Minimum required Rust version"),
            SettingItem::new("Build Profile", "Debug")
                .with_description("Current build configuration"),
            SettingItem::new("Features", "tui, cli").with_description("Enabled cargo features"),
        ];

        let database_settings = vec![
            SettingItem::new("Host", "localhost").with_description("PostgreSQL server host"),
            SettingItem::new("Port", "5432").with_description("PostgreSQL server port"),
            SettingItem::new("Database", "botticelli").with_description("Database name"),
            SettingItem::new("Status", "Not Connected")
                .with_description("Current connection status"),
            SettingItem::new("Pool Size", "10").with_description("Connection pool size"),
        ];

        let mcp_settings = vec![
            SettingItem::new("Server URL", "Not configured")
                .with_description("MCP server endpoint"),
            SettingItem::new("Status", "Disconnected").with_description("MCP connection status"),
            SettingItem::new("Tools Registered", "0")
                .with_description("Number of registered MCP tools"),
        ];

        let observability_settings = vec![
            SettingItem::new("Tracing Level", "INFO").with_description("Logging verbosity level"),
            SettingItem::new("Exporter", "stdout").with_description("Trace export backend"),
            SettingItem::new("Sampling Rate", "1.0").with_description("Trace sampling ratio"),
        ];

        let narratives_settings = vec![
            SettingItem::new("Directory", "./narratives")
                .with_description("Path to narratives directory"),
            SettingItem::new("Auto-reload", "true")
                .with_description("Automatically detect new narratives"),
            SettingItem::new("Total Narratives", "0")
                .with_description("Number of narratives found"),
        ];

        let interface_settings = vec![
            SettingItem::new("Theme", "Default").with_description("Color theme"),
            SettingItem::new("Refresh Rate", "60 FPS").with_description("Terminal refresh rate"),
            SettingItem::new("Show Help", "true").with_description("Show help text in tabs"),
        ];

        Self {
            selected_category: SettingsCategory::System,
            system_settings,
            database_settings,
            mcp_settings,
            observability_settings,
            narratives_settings,
            interface_settings,
            #[cfg(feature = "tui")]
            category_list_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
            #[cfg(feature = "tui")]
            setting_list_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
            detail_scroll: 0,
        }
    }

    /// Gets the settings for the currently selected category
    fn current_settings(&self) -> &Vec<SettingItem> {
        match self.selected_category {
            SettingsCategory::System => &self.system_settings,
            SettingsCategory::Database => &self.database_settings,
            SettingsCategory::Mcp => &self.mcp_settings,
            SettingsCategory::Observability => &self.observability_settings,
            SettingsCategory::Narratives => &self.narratives_settings,
            SettingsCategory::Interface => &self.interface_settings,
        }
    }

    /// Gets the currently selected setting
    fn selected_setting(&self) -> Option<&SettingItem> {
        #[cfg(feature = "tui")]
        {
            let settings = self.current_settings();
            self.setting_list_state
                .selected()
                .and_then(|i| settings.get(i))
        }

        #[cfg(not(feature = "tui"))]
        {
            self.current_settings().first()
        }
    }

    /// Moves category selection up
    pub fn select_previous_category(&mut self) {
        let categories = SettingsCategory::all();
        let current_idx = categories
            .iter()
            .position(|c| *c == self.selected_category)
            .unwrap_or(0);

        let new_idx = if current_idx == 0 {
            categories.len() - 1
        } else {
            current_idx - 1
        };

        self.selected_category = categories[new_idx];

        #[cfg(feature = "tui")]
        {
            self.category_list_state.select(Some(new_idx));
            self.setting_list_state.select(Some(0));
        }
        self.detail_scroll = 0;
    }

    /// Moves category selection down
    pub fn select_next_category(&mut self) {
        let categories = SettingsCategory::all();
        let current_idx = categories
            .iter()
            .position(|c| *c == self.selected_category)
            .unwrap_or(0);

        let new_idx = if current_idx >= categories.len() - 1 {
            0
        } else {
            current_idx + 1
        };

        self.selected_category = categories[new_idx];

        #[cfg(feature = "tui")]
        {
            self.category_list_state.select(Some(new_idx));
            self.setting_list_state.select(Some(0));
        }
        self.detail_scroll = 0;
    }

    /// Moves setting selection up within category
    #[cfg(feature = "tui")]
    pub fn select_previous_setting(&mut self) {
        let settings = self.current_settings();
        if settings.is_empty() {
            return;
        }

        let current = self.setting_list_state.selected().unwrap_or(0);
        let new_idx = if current == 0 {
            settings.len() - 1
        } else {
            current - 1
        };

        self.setting_list_state.select(Some(new_idx));
        self.detail_scroll = 0;
    }

    /// Moves setting selection down within category
    #[cfg(feature = "tui")]
    pub fn select_next_setting(&mut self) {
        let settings = self.current_settings();
        if settings.is_empty() {
            return;
        }

        let current = self.setting_list_state.selected().unwrap_or(0);
        let new_idx = if current >= settings.len() - 1 {
            0
        } else {
            current + 1
        };

        self.setting_list_state.select(Some(new_idx));
        self.detail_scroll = 0;
    }

    /// Scrolls detail view up
    pub fn scroll_up(&mut self) {
        if self.detail_scroll > 0 {
            self.detail_scroll -= 1;
        }
    }

    /// Scrolls detail view down
    pub fn scroll_down(&mut self) {
        self.detail_scroll += 1;
    }

    /// Handles keyboard input
    #[cfg(feature = "tui")]
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Left | KeyCode::Char('h') => {
                self.select_previous_category();
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.select_next_category();
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous_setting();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next_setting();
                true
            }
            KeyCode::PageUp => {
                self.scroll_up();
                true
            }
            KeyCode::PageDown => {
                self.scroll_down();
                true
            }
            _ => false,
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the settings tab
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Split into categories and settings/details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25),     // Categories
                Constraint::Percentage(75), // Settings & details
            ])
            .split(area);

        // Render category list
        self.render_categories(chunks[0], buf);

        // Split right side into settings list and details
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45), // Settings list
                Constraint::Percentage(50), // Setting details
                Constraint::Length(3),      // Help
            ])
            .split(chunks[1]);

        // Render settings list
        self.render_settings_list(right_chunks[0], buf);

        // Render setting details
        if let Some(setting) = self.selected_setting() {
            self.render_setting_details(setting, right_chunks[1], buf);
        } else {
            self.render_empty_details(right_chunks[1], buf);
        }

        // Render help
        self.render_help(right_chunks[2], buf);
    }

    #[cfg(feature = "tui")]
    /// Renders the category list
    fn render_categories(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::StatefulWidget;

        let items: Vec<ListItem> = SettingsCategory::all()
            .iter()
            .map(|category| {
                let line = Line::from(vec![
                    Span::raw(format!("{} ", category.icon())),
                    Span::raw(category.name()),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Categories"))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, area, buf, &mut self.category_list_state);
    }

    #[cfg(feature = "tui")]
    /// Renders the settings list for the current category
    fn render_settings_list(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::{StatefulWidget, Widget};

        let category_name = self.selected_category.name();
        let settings = self.current_settings();

        if settings.is_empty() {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from("No settings in this category"),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} Settings", category_name)),
            )
            .style(Style::default().fg(Color::DarkGray));

            Widget::render(empty, area, buf);
            return;
        }

        // Clone the settings to avoid borrow conflict
        let settings_clone: Vec<(String, String)> = settings
            .iter()
            .map(|s| (s.key.clone(), s.value.clone()))
            .collect();

        let items: Vec<ListItem> = settings_clone
            .iter()
            .map(|(key, value)| {
                let lines = vec![
                    Line::from(vec![Span::styled(
                        key,
                        Style::default().add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(vec![Span::styled(
                        format!("  {}", value),
                        Style::default().fg(Color::Cyan),
                    )]),
                ];
                ListItem::new(lines)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} Settings", category_name)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, area, buf, &mut self.setting_list_state);
    }

    #[cfg(feature = "tui")]
    /// Renders details for a single setting
    fn render_setting_details(&self, setting: &SettingItem, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Setting: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&setting.key),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Value: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&setting.value, Style::default().fg(Color::Cyan)),
            ]),
        ];

        if let Some(desc) = &setting.description {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Description: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]));

            // Wrap description
            for (i, line) in desc.lines().enumerate().skip(self.detail_scroll) {
                if lines.len() > area.height as usize - 6 {
                    break;
                }
                if i == 0 {
                    lines.push(Line::from(line.to_string()));
                } else {
                    lines.push(Line::from(format!("  {}", line)));
                }
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Editable: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                if setting.read_only { "No" } else { "Yes" },
                Style::default().fg(if setting.read_only {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]));

        let details = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Setting Details"),
            )
            .wrap(Wrap { trim: false });

        Widget::render(details, area, buf);
    }

    #[cfg(feature = "tui")]
    /// Renders empty details state
    fn render_empty_details(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;

        let empty = Paragraph::new(vec![Line::from(""), Line::from("No setting selected")])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Setting Details"),
            )
            .style(Style::default().fg(Color::DarkGray));

        Widget::render(empty, area, buf);
    }

    #[cfg(feature = "tui")]
    /// Renders help text
    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;

        let help = Paragraph::new(
            "←→/hl: Switch Category | ↑↓/jk: Navigate Settings | PgUp/PgDn: Scroll Details",
        );
        Widget::render(help, area, buf);
    }
}

impl Default for SettingsTab {
    fn default() -> Self {
        Self::new()
    }
}
