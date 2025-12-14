//! Schedule tab implementation.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::tui::Message;

pub struct ScheduleTab {
    list_state: ListState,
    scheduled_posts: Vec<ScheduledPost>,
}

impl ScheduleTab {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            list_state,
            scheduled_posts: Vec::new(),
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if self.scheduled_posts.is_empty() {
            let placeholder = Paragraph::new(vec![
                Line::from(Span::styled(
                    "No scheduled posts",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from("[N] Create new schedule"),
            ])
            .block(
                Block::default()
                    .title("Schedule")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)),
            );

            placeholder.render(area, buf);
            return;
        }

        let items: Vec<ListItem> = self
            .scheduled_posts
            .iter()
            .map(|post| {
                let content = vec![
                    Line::from(vec![
                        Span::styled(
                            &post.time,
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" - "),
                        Span::raw(&post.bot_name),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "  Platform: ",
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw(&post.platform),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "  Narrative: ",
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw(&post.narrative),
                    ]),
                ];
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Scheduled Posts")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            );

        ratatui::widgets::StatefulWidget::render(list, area, buf, &mut self.list_state);
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
        if self.scheduled_posts.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.scheduled_posts.len() - 1 {
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
        if self.scheduled_posts.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.scheduled_posts.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

struct ScheduledPost {
    time: String,
    bot_name: String,
    platform: String,
    narrative: String,
}
