// Schedule tab implementation

use chrono::{DateTime, Utc};

#[cfg(feature = "tui")]
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is enabled and active
    Active,
    /// Task is paused
    Paused,
    /// Task is disabled
    Disabled,
    /// Task failed and requires attention
    Failed,
}

/// Schedule type
#[derive(Debug, Clone)]
pub enum TaskSchedule {
    /// Fixed interval (in seconds)
    Interval(u64),
    /// Run immediately on startup
    Immediate,
    /// Cron expression (future feature)
    Cron(String),
}

impl TaskSchedule {
    /// Gets a display string for the schedule
    pub fn display(&self) -> String {
        match self {
            TaskSchedule::Interval(seconds) => {
                if *seconds < 60 {
                    format!("Every {} seconds", seconds)
                } else if *seconds < 3600 {
                    format!("Every {} minutes", seconds / 60)
                } else if *seconds < 86400 {
                    format!("Every {} hours", seconds / 3600)
                } else {
                    format!("Every {} days", seconds / 86400)
                }
            }
            TaskSchedule::Immediate => "On startup".to_string(),
            TaskSchedule::Cron(expr) => format!("Cron: {}", expr),
        }
    }
}

/// Scheduled task information
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Task unique identifier
    pub id: String,
    /// Task name/description
    pub name: String,
    /// Associated narrative name
    pub narrative: Option<String>,
    /// Associated bot name
    pub bot: Option<String>,
    /// Schedule configuration
    pub schedule: TaskSchedule,
    /// Current status
    pub status: TaskStatus,
    /// Last execution time
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled run time
    pub next_run: Option<DateTime<Utc>>,
    /// Consecutive failures
    pub failures: i32,
    /// Last error message
    pub last_error: Option<String>,
}

impl ScheduledTask {
    /// Creates a new scheduled task
    pub fn new(id: String, name: String, schedule: TaskSchedule) -> Self {
        Self {
            id,
            name,
            narrative: None,
            bot: None,
            schedule,
            status: TaskStatus::Active,
            last_run: None,
            next_run: None,
            failures: 0,
            last_error: None,
        }
    }
}

/// State for the Schedule tab
#[derive(Debug)]
pub struct ScheduleTab {
    /// List of scheduled tasks
    tasks: Vec<ScheduledTask>,

    /// Currently selected task index
    selected_index: usize,

    /// List state for navigation
    #[cfg(feature = "tui")]
    list_state: ListState,

    /// Scroll offset for details view
    detail_scroll: usize,
}

impl ScheduleTab {
    /// Creates a new schedule tab
    pub fn new() -> Self {
        // Create some placeholder tasks for demonstration
        let tasks = vec![
            ScheduledTask {
                id: "task1".to_string(),
                name: "Daily Content Generation".to_string(),
                narrative: Some("daily_posts".to_string()),
                bot: Some("discord-bot".to_string()),
                schedule: TaskSchedule::Interval(3600),
                status: TaskStatus::Active,
                last_run: Some(Utc::now() - chrono::Duration::hours(1)),
                next_run: Some(Utc::now() + chrono::Duration::minutes(55)),
                failures: 0,
                last_error: None,
            },
            ScheduledTask {
                id: "task2".to_string(),
                name: "Weekly Summary".to_string(),
                narrative: Some("weekly_digest".to_string()),
                bot: Some("twitter-bot".to_string()),
                schedule: TaskSchedule::Interval(604800),
                status: TaskStatus::Paused,
                last_run: Some(Utc::now() - chrono::Duration::days(6)),
                next_run: Some(Utc::now() + chrono::Duration::days(1)),
                failures: 0,
                last_error: None,
            },
        ];

        Self {
            tasks,
            selected_index: 0,
            #[cfg(feature = "tui")]
            list_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
            detail_scroll: 0,
        }
    }

    /// Sets the list of scheduled tasks
    pub fn set_tasks(&mut self, tasks: Vec<ScheduledTask>) {
        self.tasks = tasks;
        if !self.tasks.is_empty() && self.selected_index >= self.tasks.len() {
            self.selected_index = 0;
            #[cfg(feature = "tui")]
            {
                self.list_state.select(Some(0));
            }
        }
    }

    /// Gets the currently selected task
    pub fn selected_task(&self) -> Option<&ScheduledTask> {
        self.tasks.get(self.selected_index)
    }

    /// Gets the currently selected task mutably
    pub fn selected_task_mut(&mut self) -> Option<&mut ScheduledTask> {
        self.tasks.get_mut(self.selected_index)
    }

    /// Moves selection up
    pub fn select_previous(&mut self) {
        if self.tasks.is_empty() {
            return;
        }

        if self.selected_index == 0 {
            self.selected_index = self.tasks.len() - 1;
        } else {
            self.selected_index -= 1;
        }

        #[cfg(feature = "tui")]
        {
            self.list_state.select(Some(self.selected_index));
        }
        self.detail_scroll = 0;
    }

    /// Moves selection down
    pub fn select_next(&mut self) {
        if self.tasks.is_empty() {
            return;
        }

        if self.selected_index >= self.tasks.len() - 1 {
            self.selected_index = 0;
        } else {
            self.selected_index += 1;
        }

        #[cfg(feature = "tui")]
        {
            self.list_state.select(Some(self.selected_index));
        }
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

    /// Toggles the selected task's enabled/paused status
    pub fn toggle_selected_task(&mut self) {
        if let Some(task) = self.selected_task_mut() {
            task.status = match task.status {
                TaskStatus::Active => TaskStatus::Paused,
                TaskStatus::Paused => TaskStatus::Active,
                TaskStatus::Disabled => TaskStatus::Active,
                TaskStatus::Failed => TaskStatus::Active,
            };
        }
    }

    /// Runs the selected task immediately
    pub fn run_selected_task(&mut self) {
        if let Some(_task) = self.selected_task() {
            // TODO: Trigger task execution
            // For now, this is a placeholder
        }
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
            KeyCode::PageUp => {
                self.scroll_up();
                true
            }
            KeyCode::PageDown => {
                self.scroll_down();
                true
            }
            KeyCode::Char(' ') | KeyCode::Char('p') => {
                self.toggle_selected_task();
                true
            }
            KeyCode::Char('r') | KeyCode::Enter => {
                self.run_selected_task();
                true
            }
            _ => false,
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the schedule tab
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Split into task list and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(45), // Task list
                Constraint::Percentage(55), // Task details
            ])
            .split(area);

        // Render task list
        self.render_task_list(chunks[0], buf);

        // Render task details
        if let Some(task) = self.selected_task() {
            self.render_task_details(task, chunks[1], buf);
        } else {
            self.render_empty_state(chunks[1], buf);
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the task list
    fn render_task_list(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::{StatefulWidget, Widget};

        if self.tasks.is_empty() {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from("No scheduled tasks"),
                Line::from(""),
                Line::from("Configure tasks in Settings"),
            ])
            .block(Block::default().borders(Borders::ALL).title("Schedule"))
            .style(Style::default().fg(Color::DarkGray));

            Widget::render(empty, area, buf);
            return;
        }

        // Split into list and help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // List
                Constraint::Length(3), // Help
            ])
            .split(area);

        // Create list items
        let items: Vec<ListItem> = self
            .tasks
            .iter()
            .map(|task| {
                let (status_symbol, status_color) = match task.status {
                    TaskStatus::Active => ("▶", Color::Green),
                    TaskStatus::Paused => ("⏸", Color::Yellow),
                    TaskStatus::Disabled => ("⏹", Color::DarkGray),
                    TaskStatus::Failed => ("✗", Color::Red),
                };

                let next_run_str = match task.next_run {
                    Some(next) => {
                        let duration = next.signed_duration_since(Utc::now());
                        if duration.num_seconds() < 0 {
                            "overdue".to_string()
                        } else if duration.num_hours() < 1 {
                            format!("{}m", duration.num_minutes())
                        } else if duration.num_days() < 1 {
                            format!("{}h", duration.num_hours())
                        } else {
                            format!("{}d", duration.num_days())
                        }
                    }
                    None => "—".to_string(),
                };

                let lines = vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", status_symbol),
                            Style::default().fg(status_color),
                        ),
                        Span::styled(&task.name, Style::default().add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(task.schedule.display(), Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!(" • next: {}", next_run_str),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                ];

                ListItem::new(lines)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Scheduled Tasks ({} total)", self.tasks.len())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, chunks[0], buf, &mut self.list_state);

        // Render help
        let help = Paragraph::new("↑↓/jk: Navigate | Space/p: Pause/Resume | r/Enter: Run Now");
        Widget::render(help, chunks[1], buf);
    }

    #[cfg(feature = "tui")]
    /// Renders task details
    fn render_task_details(&self, task: &ScheduledTask, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;

        let status_str = match task.status {
            TaskStatus::Active => "Active",
            TaskStatus::Paused => "Paused",
            TaskStatus::Disabled => "Disabled",
            TaskStatus::Failed => "Failed",
        };

        let status_color = match task.status {
            TaskStatus::Active => Color::Green,
            TaskStatus::Paused => Color::Yellow,
            TaskStatus::Disabled => Color::DarkGray,
            TaskStatus::Failed => Color::Red,
        };

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Task: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(status_str, Style::default().fg(status_color)),
            ]),
            Line::from(vec![
                Span::styled("Schedule: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(task.schedule.display()),
            ]),
        ];

        if let Some(narrative) = &task.narrative {
            lines.push(Line::from(vec![
                Span::styled("Narrative: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(narrative),
            ]));
        }

        if let Some(bot) = &task.bot {
            lines.push(Line::from(vec![
                Span::styled("Bot: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(bot),
            ]));
        }

        lines.push(Line::from(""));

        if let Some(last) = task.last_run {
            let ago = Utc::now().signed_duration_since(last);
            let ago_str = if ago.num_hours() < 1 {
                format!("{} minutes ago", ago.num_minutes())
            } else if ago.num_days() < 1 {
                format!("{} hours ago", ago.num_hours())
            } else {
                format!("{} days ago", ago.num_days())
            };

            lines.push(Line::from(vec![
                Span::styled("Last run: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(
                    "{} ({})",
                    last.format("%Y-%m-%d %H:%M:%S"),
                    ago_str
                )),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Last run: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Never", Style::default().fg(Color::DarkGray)),
            ]));
        }

        if let Some(next) = task.next_run {
            let until = next.signed_duration_since(Utc::now());
            let until_str = if until.num_seconds() < 0 {
                "Overdue".to_string()
            } else if until.num_hours() < 1 {
                format!("In {} minutes", until.num_minutes())
            } else if until.num_days() < 1 {
                format!("In {} hours", until.num_hours())
            } else {
                format!("In {} days", until.num_days())
            };

            let color = if until.num_seconds() < 0 {
                Color::Red
            } else {
                Color::White
            };

            lines.push(Line::from(vec![
                Span::styled("Next run: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{} ({})", next.format("%Y-%m-%d %H:%M:%S"), until_str),
                    Style::default().fg(color),
                ),
            ]));
        }

        lines.push(Line::from(""));

        if task.failures > 0 {
            lines.push(Line::from(vec![
                Span::styled("Failures: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(task.failures.to_string(), Style::default().fg(Color::Red)),
            ]));
        }

        if let Some(error) = &task.last_error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Last Error: ",
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
            )]));

            // Wrap error message
            for line in error.lines().skip(self.detail_scroll) {
                if lines.len() > area.height as usize - 4 {
                    break;
                }
                lines.push(Line::from(format!("  {}", line)));
            }
        }

        let details = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Task Details"))
            .wrap(Wrap { trim: false });

        Widget::render(details, area, buf);
    }

    #[cfg(feature = "tui")]
    /// Renders empty state
    fn render_empty_state(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;

        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from("No task selected"),
            Line::from(""),
            Line::from("Use arrow keys or j/k to select a task"),
        ])
        .block(Block::default().borders(Borders::ALL).title("Task Details"))
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: false });

        Widget::render(empty, area, buf);
    }
}

impl Default for ScheduleTab {
    fn default() -> Self {
        Self::new()
    }
}
