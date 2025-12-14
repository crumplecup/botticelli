//! Database tab implementation.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Row, Table},
};

use crate::tui::Message;

pub struct DatabaseTab {
    list_state: ListState,
    tables: Vec<TableInfo>,
    selected_table_data: Option<Vec<Vec<String>>>,
}

impl DatabaseTab {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let tables = vec![
            TableInfo {
                name: "content".to_string(),
                row_count: 0,
            },
            TableInfo {
                name: "narrative_executions".to_string(),
                row_count: 0,
            },
            TableInfo {
                name: "act_executions".to_string(),
                row_count: 0,
            },
            TableInfo {
                name: "model_responses".to_string(),
                row_count: 0,
            },
            TableInfo {
                name: "content_generations".to_string(),
                row_count: 0,
            },
        ];

        Self {
            list_state,
            tables,
            selected_table_data: None,
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);

        self.render_table_list(chunks[0], buf);
        self.render_table_data(chunks[1], buf);
    }

    fn render_table_list(&mut self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .tables
            .iter()
            .map(|table| {
                let content = Line::from(vec![
                    Span::raw(&table.name),
                    Span::styled(
                        format!(" ({})", table.row_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Tables")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        ratatui::widgets::StatefulWidget::render(list, area, buf, &mut self.list_state);
    }

    fn render_table_data(&self, area: Rect, buf: &mut Buffer) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(table) = self.tables.get(selected) {
                if let Some(data) = &self.selected_table_data {
                    if !data.is_empty() {
                        let header = Row::new(data[0].iter().map(|h| h.as_str()))
                            .style(Style::default().add_modifier(Modifier::BOLD))
                            .bottom_margin(1);

                        let rows = data.iter().skip(1).map(|row| {
                            Row::new(row.iter().map(|cell| cell.as_str()))
                        });

                        let widths = vec![Constraint::Percentage(25); data[0].len()];

                        let table_widget = Table::new(rows, widths)
                            .header(header)
                            .block(
                                Block::default()
                                    .title(format!("Table: {}", table.name))
                                    .borders(Borders::ALL)
                                    .border_style(Style::default().fg(Color::Cyan)),
                            )
                            .column_spacing(1);

                        ratatui::widgets::Widget::render(table_widget, area, buf);
                        return;
                    }
                }

                let placeholder = Paragraph::new(vec![
                    Line::from("No data loaded"),
                    Line::from(""),
                    Line::from("[Enter] Load data"),
                    Line::from("[Q] Query editor"),
                ])
                .block(
                    Block::default()
                        .title(format!("Table: {}", table.name))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                );

                placeholder.render(area, buf);
            }
        }
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
        if self.tables.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.tables.len() - 1 {
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
        if self.tables.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tables.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

struct TableInfo {
    name: String,
    row_count: usize,
}
