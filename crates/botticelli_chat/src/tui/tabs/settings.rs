//! Settings tab implementation.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::tui::Message;

pub struct SettingsTab {
    list_state: ListState,
    categories: Vec<SettingCategory>,
}

impl SettingsTab {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let categories = vec![
            SettingCategory {
                name: "LLM Provider".to_string(),
                settings: vec![
                    Setting {
                        key: "Provider".to_string(),
                        value: "Anthropic".to_string(),
                    },
                    Setting {
                        key: "Model".to_string(),
                        value: "claude-3-5-sonnet-20241022".to_string(),
                    },
                    Setting {
                        key: "Temperature".to_string(),
                        value: "0.7".to_string(),
                    },
                ],
            },
            SettingCategory {
                name: "Database".to_string(),
                settings: vec![
                    Setting {
                        key: "Host".to_string(),
                        value: "localhost".to_string(),
                    },
                    Setting {
                        key: "Port".to_string(),
                        value: "5432".to_string(),
                    },
                    Setting {
                        key: "Status".to_string(),
                        value: "Connected".to_string(),
                    },
                ],
            },
            SettingCategory {
                name: "MCP".to_string(),
                settings: vec![
                    Setting {
                        key: "Status".to_string(),
                        value: "Connected".to_string(),
                    },
                    Setting {
                        key: "Tools".to_string(),
                        value: "29 available".to_string(),
                    },
                ],
            },
        ];

        Self {
            list_state,
            categories,
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        self.render_category_list(chunks[0], buf);
        self.render_settings_detail(chunks[1], buf);
    }

    fn render_category_list(&mut self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .categories
            .iter()
            .map(|cat| ListItem::new(Line::from(&cat.name)))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Settings")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            );

        ratatui::widgets::StatefulWidget::render(list, area, buf, &mut self.list_state);
    }

    fn render_settings_detail(&self, area: Rect, buf: &mut Buffer) {
        let content = if let Some(selected) = self.list_state.selected() {
            if let Some(category) = self.categories.get(selected) {
                let mut lines = vec![Line::from(vec![Span::styled(
                    &category.name,
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )])];

                lines.push(Line::from(""));

                for setting in &category.settings {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("{}: ", setting.key),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(&setting.value),
                    ]));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "[E] Edit  [T] Test Connection  [S] Save",
                    Style::default().fg(Color::DarkGray),
                )));

                lines
            } else {
                vec![Line::from("No category selected")]
            }
        } else {
            vec![Line::from("No categories available")]
        };

        let paragraph = Paragraph::new(content).block(
            Block::default()
                .title("Details")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        paragraph.render(area, buf);
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<Message> {
        match key {
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                self.previous();
                None
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                self.next();
                None
            }
            _ => None,
        }
    }

    fn next(&mut self) {
        if self.categories.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.categories.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.categories.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.categories.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

struct SettingCategory {
    name: String,
    settings: Vec<Setting>,
}

struct Setting {
    key: String,
    value: String,
}
