// Bots tab implementation

#[cfg(feature = "tui")]
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

/// Bot status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotStatus {
    /// Bot is running
    Running,
    /// Bot is stopped
    Stopped,
    /// Bot configuration only
    Configured,
}

/// Bot information for display
#[derive(Debug, Clone)]
pub struct BotInfo {
    /// Bot name
    pub name: String,
    /// Bot description
    pub description: Option<String>,
    /// Platform
    pub platform: String,
    /// Current status
    pub status: BotStatus,
    /// Configuration path
    pub config_path: Option<String>,
}

impl BotInfo {
    /// Creates a new bot info
    pub fn new(name: String, platform: String, status: BotStatus) -> Self {
        Self {
            name,
            description: None,
            platform,
            status,
            config_path: None,
        }
    }
}

/// State for the Bots tab
#[derive(Debug)]
pub struct BotsTab {
    /// List of bots
    bots: Vec<BotInfo>,

    /// List state for selection
    #[cfg(feature = "tui")]
    list_state: ListState,
}

impl BotsTab {
    /// Creates a new bots tab
    pub fn new() -> Self {
        // For now, create some placeholder bots
        let bots = vec![BotInfo {
            name: "demo-bot".to_string(),
            description: Some("Demo bot for testing".to_string()),
            platform: "Discord".to_string(),
            status: BotStatus::Configured,
            config_path: Some("config/demo_bot.toml".to_string()),
        }];

        Self {
            bots,
            #[cfg(feature = "tui")]
            list_state: ListState::default(),
        }
    }

    /// Adds a bot
    pub fn add_bot(&mut self, bot: BotInfo) {
        self.bots.push(bot);
    }

    /// Gets the currently selected bot
    pub fn selected_bot(&self) -> Option<&BotInfo> {
        #[cfg(feature = "tui")]
        {
            self.list_state.selected().and_then(|i| self.bots.get(i))
        }

        #[cfg(not(feature = "tui"))]
        {
            self.bots.first()
        }
    }

    /// Moves selection up
    #[cfg(feature = "tui")]
    pub fn select_previous(&mut self) {
        if self.bots.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.bots.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Moves selection down
    #[cfg(feature = "tui")]
    pub fn select_next(&mut self) {
        if self.bots.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.bots.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Handles keyboard input
    #[cfg(feature = "tui")]
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                true
            }
            KeyCode::Char('s') => {
                // TODO: Start bot
                true
            }
            KeyCode::Char('x') => {
                // TODO: Stop bot
                true
            }
            KeyCode::Char('r') => {
                // TODO: Restart bot
                true
            }
            _ => false,
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the bots tab
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Split into list and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Bot list
                Constraint::Percentage(60), // Bot details
            ])
            .split(area);

        // Render bot list
        self.render_bot_list(chunks[0], buf);

        // Render bot details
        if let Some(bot) = self.selected_bot() {
            self.render_bot_details(bot, chunks[1], buf);
        } else {
            self.render_empty_state(chunks[1], buf);
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the bot list
    fn render_bot_list(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::{StatefulWidget, Widget};

        if self.bots.is_empty() {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from("No bots configured"),
                Line::from(""),
                Line::from("Add bots in the Settings tab"),
            ])
            .block(Block::default().borders(Borders::ALL).title("Bots"))
            .style(Style::default().fg(Color::DarkGray));

            Widget::render(empty, area, buf);
            return;
        }

        let items: Vec<ListItem> = self
            .bots
            .iter()
            .map(|bot| {
                let status_symbol = match bot.status {
                    BotStatus::Running => "●",
                    BotStatus::Stopped => "○",
                    BotStatus::Configured => "◎",
                };

                let status_color = match bot.status {
                    BotStatus::Running => Color::Green,
                    BotStatus::Stopped => Color::Red,
                    BotStatus::Configured => Color::Yellow,
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", status_symbol),
                        Style::default().fg(status_color),
                    ),
                    Span::raw(&bot.name),
                    Span::styled(
                        format!(" ({})", bot.platform),
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
                    .title(format!("Bots ({} configured)", self.bots.len())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }

    #[cfg(feature = "tui")]
    /// Renders bot details
    fn render_bot_details(&self, bot: &BotInfo, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;

        let status_str = match bot.status {
            BotStatus::Running => "Running",
            BotStatus::Stopped => "Stopped",
            BotStatus::Configured => "Configured",
        };

        let status_color = match bot.status {
            BotStatus::Running => Color::Green,
            BotStatus::Stopped => Color::Red,
            BotStatus::Configured => Color::Yellow,
        };

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&bot.name),
            ]),
            Line::from(vec![
                Span::styled("Platform: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&bot.platform),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(status_str, Style::default().fg(status_color)),
            ]),
        ];

        if let Some(desc) = &bot.description {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Description: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(desc.as_str()));
        }

        if let Some(config) = &bot.config_path {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Config: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(config),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Actions: ",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from("  s - Start bot"));
        lines.push(Line::from("  x - Stop bot"));
        lines.push(Line::from("  r - Restart bot"));

        let details = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Bot Details"))
            .wrap(Wrap { trim: false });

        Widget::render(details, area, buf);
    }

    #[cfg(feature = "tui")]
    /// Renders empty state
    fn render_empty_state(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;

        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from("No bot selected"),
            Line::from(""),
            Line::from("Use arrow keys or j/k to select a bot"),
        ])
        .block(Block::default().borders(Borders::ALL).title("Bot Details"))
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: false });

        Widget::render(empty, area, buf);
    }
}

impl Default for BotsTab {
    fn default() -> Self {
        Self::new()
    }
}
