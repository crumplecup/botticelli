//! Bots tab implementation.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::tui::{state::AppState, Message};

pub struct BotsTab {
    list_state: ListState,
    bots: Vec<BotInfo>,
}

impl BotsTab {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            list_state,
            bots: Vec::new(),
        }
    }

    pub fn refresh_bots(&mut self, _app_state: &AppState) {
        self.bots.clear();
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        self.render_bot_list(chunks[0], buf);
        self.render_bot_details(chunks[1], buf);
    }

    fn render_bot_list(&mut self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .bots
            .iter()
            .map(|bot| {
                let status_icon = if bot.is_running { "●" } else { "○" };
                let content = Line::from(vec![
                    Span::styled(
                        status_icon,
                        Style::default().fg(if bot.is_running {
                            Color::Green
                        } else {
                            Color::Gray
                        }),
                    ),
                    Span::raw(" "),
                    Span::raw(&bot.name),
                ]);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Bots")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        ratatui::widgets::StatefulWidget::render(list, area, buf, &mut self.list_state);
    }

    fn render_bot_details(&self, area: Rect, buf: &mut Buffer) {
        let content = if let Some(selected) = self.list_state.selected() {
            if let Some(bot) = self.bots.get(selected) {
                vec![
                    Line::from(vec![
                        Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(&bot.name),
                    ]),
                    Line::from(vec![
                        Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(
                            if bot.is_running { "Running" } else { "Stopped" },
                            Style::default().fg(if bot.is_running {
                                Color::Green
                            } else {
                                Color::Red
                            }),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            "Narrative: ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(bot.narrative.as_deref().unwrap_or("None")),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Platform: ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(&bot.platform),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Actions:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from("[S] Start  [P] Pause  [L] Logs"),
                    Line::from("[A] Assign Narrative  [E] Edit Config"),
                ]
            } else {
                vec![Line::from("No bot selected")]
            }
        } else {
            vec![Line::from("No bots available")]
        };

        let paragraph = Paragraph::new(content).block(
            Block::default()
                .title("Bot Details")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
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

    fn previous(&mut self) {
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
}

struct BotInfo {
    name: String,
    is_running: bool,
    narrative: Option<String>,
    platform: String,
}
